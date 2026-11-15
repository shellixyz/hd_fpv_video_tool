use std::{
	env, fs,
	path::{Path, PathBuf},
	process::Command,
	time::{Duration, Instant},
};

use hd_fpv_video_tool::osd::overlay::{
	Dimensions, OVERLAY_ALPHA_X_METADATA, OVERLAY_MODE_METADATA, OVERLAY_SIZE_METADATA, PACKED_ALPHA_MODE,
	PackedAlphaLayout,
};
use image::RgbaImage;

/// Largest acceptable difference, in 8-bit levels, between a composited source pixel channel and
/// the same channel composited from the decoded sidecar.
const COMPOSITED_CHANNEL_TOLERANCE: u32 = 24;
/// Backgrounds the overlays are composited over: the extremes expose both alpha and color errors.
const COMPOSITING_BACKGROUNDS: [u32; 2] = [0, 255];
/// Composited pixels allowed to differ, relative to the number of visible (non-transparent) source
/// pixels. 4:2:0 chroma subsampling alone accounts for about 2.5% on this fixture, from thin
/// colored strokes next to black outlines; a lossless 4:2:0 encode scores 2.45% and AV1 VAAPI 3.0%.
const MAX_DIFFERING_VISIBLE_PIXEL_RATIO: f64 = 0.04;
/// Largest acceptable alpha difference. Alpha travels in full-resolution luma, so it is unaffected
/// by chroma subsampling.
const ALPHA_TOLERANCE: u8 = 24;
/// Pixels allowed to exceed [`ALPHA_TOLERANCE`], relative to the number of visible source pixels.
const MAX_ALPHA_DIFFERING_VISIBLE_PIXEL_RATIO: f64 = 0.001;
/// Largest acceptable mean alpha error over fully transparent and fully opaque source pixels. Alpha
/// decoded without expanding limited-range luma would be off by about 16 and 20 levels.
const MAX_MEAN_ALPHA_RANGE_ERROR: f64 = 4.0;
const VALIDATION_END_TIMESTAMP: &str = "00:01:00";
const COMPARISON_END_TIMESTAMP: &str = "00:00:01";

fn fixture_dir() -> PathBuf {
	env::var_os("OVERLAY_VIDEO_QUALITY_FIXTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn font_dir() -> PathBuf {
	PathBuf::from(
		env::var_os("OVERLAY_VIDEO_QUALITY_FONT_DIR")
			.expect("set OVERLAY_VIDEO_QUALITY_FONT_DIR to the directory containing OSD font files"),
	)
}

fn fixture_path(fixture_dir: &Path, file_name: &str) -> PathBuf {
	let path = fixture_dir.join(file_name);
	assert!(path.is_file(), "fixture is missing: {}", path.display());
	path
}

/// Runs a command, asserting that it succeeds, and returns its standard output.
fn run_command(command: &mut Command) -> String {
	let output = command.output().expect("failed to start command");
	assert!(
		output.status.success(),
		"command failed: {command:?}\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
	String::from_utf8_lossy(&output.stdout).into_owned()
}

fn png_paths(path: &Path) -> Vec<PathBuf> {
	let mut paths = fs::read_dir(path)
		.expect("failed to read frame directory")
		.map(|entry| entry.expect("failed to read frame directory entry").path())
		.filter(|path| {
			path.extension()
				.is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
		})
		.collect::<Vec<_>>();
	paths.sort();
	assert!(!paths.is_empty(), "no frames were generated in {}", path.display());
	paths
}

fn composite(pixel: image::Rgba<u8>, channel: usize, background: u32) -> u32 {
	let alpha = u32::from(pixel[3]);
	u32::from(pixel[channel]) * alpha + background * (255 - alpha)
}

/// Whether the two overlay pixels look different when composited over any test background.
fn pixel_differs(expected: image::Rgba<u8>, actual: image::Rgba<u8>) -> bool {
	COMPOSITING_BACKGROUNDS.into_iter().any(|background| {
		(0..3).any(|channel| {
			composite(expected, channel, background).abs_diff(composite(actual, channel, background))
				> COMPOSITED_CHANNEL_TOLERANCE * 255
		})
	})
}

#[derive(Default)]
struct Comparison {
	differing: usize,
	alpha_differing: usize,
	visible: usize,
	transparent: usize,
	transparent_alpha_sum: usize,
	opaque: usize,
	opaque_alpha_deficit_sum: usize,
}

impl Comparison {
	fn of_frames(expected: &RgbaImage, actual: &RgbaImage) -> Self {
		assert_eq!(
			expected.dimensions(),
			actual.dimensions(),
			"decoded frame dimensions differ"
		);
		let mut comparison = Self::default();
		for (expected, actual) in expected.pixels().zip(actual.pixels()) {
			comparison.differing += usize::from(pixel_differs(*expected, *actual));
			comparison.alpha_differing += usize::from(expected[3].abs_diff(actual[3]) > ALPHA_TOLERANCE);
			comparison.visible += usize::from(expected[3] != 0);
			match expected[3] {
				u8::MIN => {
					comparison.transparent += 1;
					comparison.transparent_alpha_sum += usize::from(actual[3]);
				},
				u8::MAX => {
					comparison.opaque += 1;
					comparison.opaque_alpha_deficit_sum += usize::from(u8::MAX - actual[3]);
				},
				_ => {},
			}
		}
		comparison
	}

	fn add(self, other: Self) -> Self {
		Self {
			differing: self.differing + other.differing,
			alpha_differing: self.alpha_differing + other.alpha_differing,
			visible: self.visible + other.visible,
			transparent: self.transparent + other.transparent,
			transparent_alpha_sum: self.transparent_alpha_sum + other.transparent_alpha_sum,
			opaque: self.opaque + other.opaque,
			opaque_alpha_deficit_sum: self.opaque_alpha_deficit_sum + other.opaque_alpha_deficit_sum,
		}
	}

	fn ratio(count: usize, total: usize) -> f64 {
		count as f64 / total as f64
	}
}

fn recorded_layout(sidecar_file: &Path) -> PackedAlphaLayout {
	let tags = run_command(
		Command::new("ffprobe")
			.args([
				"-v",
				"error",
				"-show_entries",
				"format_tags",
				"-of",
				"default=noprint_wrappers=1",
			])
			.arg(sidecar_file),
	);
	let tag = |name: &str| {
		let prefix = format!("TAG:{name}=");
		tags.lines()
			.find_map(|line| line.strip_prefix(&prefix))
			.unwrap_or_else(|| panic!("sidecar has no {name} metadata"))
			.to_owned()
	};
	assert_eq!(
		tag(OVERLAY_MODE_METADATA),
		PACKED_ALPHA_MODE,
		"sidecar is not tagged as packed alpha"
	);
	PackedAlphaLayout::from_metadata(&tag(OVERLAY_SIZE_METADATA), &tag(OVERLAY_ALPHA_X_METADATA))
		.expect("sidecar has invalid layout metadata")
}

fn format_duration(duration: Duration) -> String {
	format!("{:.2}s", duration.as_secs_f64())
}

#[test]
#[ignore = "requires the DJIG0029 fixtures, OSD fonts, ffmpeg, and ffprobe"]
fn packed_alpha_sidecar_should_match_rendered_overlay_within_tolerance() {
	let fixture_dir = fixture_dir();
	let video_file = fixture_path(&fixture_dir, "DJIG0029_cut.mp4");
	let osd_file = fixture_path(&fixture_dir, "DJIG0029.osd");
	let font_dir = font_dir();
	assert!(font_dir.is_dir(), "font directory is missing: {}", font_dir.display());

	let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
	let source_frames_dir = temp_dir.path().join("source");
	let decoded_frames_dir = temp_dir.path().join("decoded");
	let sidecar_file = temp_dir.path().join("DJIG0029_cut_osd.mp4");
	let binary = env!("CARGO_BIN_EXE_hd_fpv_video_tool");

	run_command(
		Command::new(binary)
			.args([
				"generate-overlay-frames",
				"--end",
				COMPARISON_END_TIMESTAMP,
				"--target-video-file",
			])
			.arg(&video_file)
			.args(["--font-dir"])
			.arg(&font_dir)
			.arg(&osd_file)
			.arg(&source_frames_dir),
	);

	let started_at = Instant::now();
	run_command(
		Command::new(binary)
			.args([
				"generate-overlay-video",
				"--end",
				VALIDATION_END_TIMESTAMP,
				"--target-video-file",
			])
			.arg(&video_file)
			.args(["--font-dir"])
			.arg(&font_dir)
			.arg(&osd_file)
			.arg(&sidecar_file),
	);
	let encode_duration = started_at.elapsed();

	let source_frames = png_paths(&source_frames_dir);
	let (frame_width, frame_height) =
		image::image_dimensions(&source_frames[0]).expect("failed to read source frame dimensions");
	let layout = recorded_layout(&sidecar_file);
	assert_eq!(
		layout.overlay_dimensions(),
		Dimensions::new(frame_width, frame_height),
		"recorded overlay size differs from the rendered frames"
	);

	fs::create_dir(&decoded_frames_dir).expect("failed to create decoded-frame directory");
	run_command(
		Command::new("ffmpeg")
			.args(["-v", "error", "-i"])
			.arg(&sidecar_file)
			.args([
				"-filter_complex",
				&layout.unpack_filter("0:v", "overlay"),
				"-map",
				"[overlay]",
			])
			.args(["-pix_fmt", "rgba", "-frames:v", "60", "-start_number", "0"])
			.arg(decoded_frames_dir.join("%010d.png")),
	);

	let decoded_frames = png_paths(&decoded_frames_dir);
	assert_eq!(source_frames.len(), decoded_frames.len(), "decoded frame count differs");

	let Comparison {
		differing,
		alpha_differing,
		visible,
		transparent,
		transparent_alpha_sum,
		opaque,
		opaque_alpha_deficit_sum,
	} = source_frames
		.iter()
		.zip(&decoded_frames)
		.fold(Comparison::default(), |comparison, (source, decoded)| {
			let source = image::open(source).expect("failed to load source frame").into_rgba8();
			let decoded = image::open(decoded).expect("failed to load decoded frame").into_rgba8();
			comparison.add(Comparison::of_frames(&source, &decoded))
		});
	let differing_ratio = Comparison::ratio(differing, visible);
	let alpha_differing_ratio = Comparison::ratio(alpha_differing, visible);
	let mean_transparent_alpha = Comparison::ratio(transparent_alpha_sum, transparent);
	let mean_opaque_alpha_deficit = Comparison::ratio(opaque_alpha_deficit_sum, opaque);

	eprintln!(
		"{visible} visible source pixels: {differing} ({:.3}%) differ when composited, {alpha_differing} ({:.3}%) in \
		 alpha",
		differing_ratio * 100.0,
		alpha_differing_ratio * 100.0,
	);
	eprintln!(
		"mean decoded alpha: {mean_transparent_alpha:.3} where transparent, 255 - {mean_opaque_alpha_deficit:.3} \
		 where opaque"
	);
	assert!(
		mean_transparent_alpha <= MAX_MEAN_ALPHA_RANGE_ERROR && mean_opaque_alpha_deficit <= MAX_MEAN_ALPHA_RANGE_ERROR,
		"decoded alpha does not span the full range: {mean_transparent_alpha:.3} where transparent, 255 - \
		 {mean_opaque_alpha_deficit:.3} where opaque",
	);
	assert!(
		differing_ratio <= MAX_DIFFERING_VISIBLE_PIXEL_RATIO,
		"{:.3}% of visible pixels differ by more than {COMPOSITED_CHANNEL_TOLERANCE} levels when composited",
		differing_ratio * 100.0,
	);
	assert!(
		alpha_differing_ratio <= MAX_ALPHA_DIFFERING_VISIBLE_PIXEL_RATIO,
		"{:.3}% of visible pixels differ by more than {ALPHA_TOLERANCE} in alpha",
		alpha_differing_ratio * 100.0,
	);

	let probe_output = run_command(
		Command::new("ffprobe")
			.args([
				"-v",
				"error",
				"-select_streams",
				"v:0",
				"-show_entries",
				"stream=codec_name,bit_rate:stream_tags=encoder",
				"-of",
				"default=noprint_wrappers=1",
			])
			.arg(&sidecar_file),
	);
	eprintln!(
		"packed-alpha benchmark:\n{}\nencoded sidecar: {} bytes in {}",
		probe_output.trim(),
		fs::metadata(&sidecar_file).expect("failed to stat sidecar").len(),
		format_duration(encode_duration),
	);
}
