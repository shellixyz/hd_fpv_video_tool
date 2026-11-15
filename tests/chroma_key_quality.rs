use std::{
	env, fs,
	path::{Path, PathBuf},
	process::Command,
	time::{Duration, Instant},
};

use image::RgbaImage;

const PIXEL_CHANNEL_TOLERANCE: u8 = 24;
const MAX_DIFFERING_PIXEL_RATIO: f64 = 0.065;
const VALIDATION_END_TIMESTAMP: &str = "00:01:00";
const COMPARISON_END_TIMESTAMP: &str = "00:00:01";
const VALIDATION_KEY_COLOR: &str = "000040";

fn fixture_dir() -> PathBuf {
	env::var_os("CHROMA_KEY_QUALITY_FIXTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn font_dir() -> PathBuf {
	PathBuf::from(
		env::var_os("CHROMA_KEY_QUALITY_FONT_DIR")
			.expect("set CHROMA_KEY_QUALITY_FONT_DIR to the directory containing OSD font files"),
	)
}

fn fixture_path(fixture_dir: &Path, file_name: &str) -> PathBuf {
	let path = fixture_dir.join(file_name);
	assert!(path.is_file(), "fixture is missing: {}", path.display());
	path
}

fn run_command(command: &mut Command) {
	let output = command.output().expect("failed to start command");
	assert!(
		output.status.success(),
		"command failed: {command:?}\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
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

fn pixel_differs(expected: &image::Rgba<u8>, actual: &image::Rgba<u8>) -> bool {
	if expected[3] == 0 && actual[3] == 0 {
		return false;
	}
	expected
		.0
		.into_iter()
		.zip(actual.0)
		.any(|(expected, actual)| expected.abs_diff(actual) > PIXEL_CHANNEL_TOLERANCE)
}

fn count_differing_pixels(expected: &RgbaImage, actual: &RgbaImage) -> usize {
	assert_eq!(
		expected.dimensions(),
		actual.dimensions(),
		"decoded frame dimensions differ"
	);
	expected
		.pixels()
		.zip(actual.pixels())
		.filter(|(expected, actual)| pixel_differs(expected, actual))
		.count()
}

fn format_duration(duration: Duration) -> String {
	format!("{:.2}s", duration.as_secs_f64())
}

#[test]
#[ignore = "requires the DJIG0029 fixtures, OSD fonts, ffmpeg, and ffprobe"]
fn chroma_key_sidecar_should_match_rendered_overlay_within_pixel_tolerance() {
	let fixture_dir = fixture_dir();
	let video_file = fixture_path(&fixture_dir, "DJIG0029_cut.mp4");
	let osd_file = fixture_path(&fixture_dir, "DJIG0029.osd");
	let font_dir = font_dir();
	assert!(font_dir.is_dir(), "font directory is missing: {}", font_dir.display());

	let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
	let source_frames_dir = temp_dir.path().join("source");
	let keyed_frames_dir = temp_dir.path().join("keyed");
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
				"--key-color",
				VALIDATION_KEY_COLOR,
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
	let chroma_key_filter =
		format!("chromakey=0x{VALIDATION_KEY_COLOR}:0.05:0.0,crop={frame_width}:{frame_height}:(iw-ow)/2:(ih-oh)/2");

	fs::create_dir(&keyed_frames_dir).expect("failed to create decoded-frame directory");
	run_command(
		Command::new("ffmpeg")
			.args(["-v", "error", "-i"])
			.arg(&sidecar_file)
			.args(["-vf", &chroma_key_filter, "-frames:v", "60", "-start_number", "0"])
			.arg(keyed_frames_dir.join("%010d.png")),
	);

	let keyed_frames = png_paths(&keyed_frames_dir);
	assert_eq!(source_frames.len(), keyed_frames.len(), "decoded frame count differs");

	let (differing_pixels, total_pixels) = source_frames.iter().zip(&keyed_frames).fold(
		(0_usize, 0_usize),
		|(differing_pixels, total_pixels), (source, keyed)| {
			let source = image::open(source).expect("failed to load source frame").into_rgba8();
			let keyed = image::open(keyed).expect("failed to load keyed frame").into_rgba8();
			(
				differing_pixels + count_differing_pixels(&source, &keyed),
				total_pixels + (source.width() * source.height()) as usize,
			)
		},
	);
	let differing_pixel_ratio = differing_pixels as f64 / total_pixels as f64;

	assert!(
		differing_pixel_ratio <= MAX_DIFFERING_PIXEL_RATIO,
		"{differing_pixels}/{total_pixels} pixels ({:.3}%) differ by more than {PIXEL_CHANNEL_TOLERANCE} in a channel",
		differing_pixel_ratio * 100.0,
	);

	let probe_output = Command::new("ffprobe")
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
		.arg(&sidecar_file)
		.output()
		.expect("failed to start ffprobe");
	assert!(
		probe_output.status.success(),
		"ffprobe failed:\n{}",
		String::from_utf8_lossy(&probe_output.stderr),
	);
	eprintln!(
		"chroma-key benchmark:\n{}\nencoded sidecar: {} bytes in {}",
		String::from_utf8_lossy(&probe_output.stdout).trim(),
		fs::metadata(&sidecar_file).expect("failed to stat sidecar").len(),
		format_duration(encode_duration),
	);
}
