# 05: Fall back to software encoding when the frame exceeds the hardware encoder's limits

**What to build:** when the default encoder would be a VAAPI hardware encoder but the frames to
encode are larger than it supports, encode in software (libx265) and log why, instead of letting
FFmpeg fail partway through. The case that motivates it is a packed-alpha sidecar: its frame is
`align64(W) + W` wide (`PackedAlphaLayout`, `src/osd/overlay.rs`), so an overlay above about
2048 px wide, as when scaling to a 4K target, exceeds the 4096-wide limit common to hardware
encoders.

**Status:** done

## Decisions

Settled with the user on 2026-11-16.

- **Worth doing.** The user renders overlays above 2048 px wide, so packed-alpha sidecars go past
  the 4096 px hardware limit.
- **The device reports the limit.** Read `VAConfigAttribMaxPictureWidth` and
  `VAConfigAttribMaxPictureHeight` for the codec's profile and encode entrypoint through
  `cros_libva::Display::get_config_attributes`, which takes any attribute; the generated bindings
  include both. When the driver reports neither (`VA_ATTRIB_NOT_SUPPORTED`), treat the size as
  unlimited, as today.
- **An explicit codec too large for hardware falls back to that codec's software encoder** with
  a warning naming the frame size and the limit, so the user still gets the codec they asked for.
- **Every encode is covered**: packed-alpha sidecars and plain transcodes alike, with the size of
  the frames actually encoded (after scaling).
- **Merge first.** One function replaces `video::default_video_codec`,
  `TranscodeConfig::resolve_codec` and `TranscodeVideoArgs::video_codec`, as a slice that keeps
  current behaviour. The size check is then added once, in that function.

## Seams under test

- A pure selection function taking the requested codec, `no_hwaccel`, the frame size, and an
  encoder-capabilities trait that reports, per codec, either "cannot encode" or a maximum size.
  Tests use a fake: default and explicit codec, each against fits, too wide, too tall, no hardware,
  and no reported limit, plus the reason logged on fallback. `VaapiCapFinder` implements the trait;
  its VA-API query needs real hardware and stays untested.
- Callers pass the right frame size: the packed-alpha frame width (`align64(W) + W`) for
  sidecars, and the scaled output size for transcodes.

## Checklist

- [x] Merge the three selection copies into one function behind an encoder-capabilities trait,
      keeping current behaviour.
- [x] Query the hardware encoder's maximum frame size per codec from VA-API.
- [x] When choosing the default encoder, skip hardware codecs whose maximum the frame exceeds, and
      fall back to software H.265 with a log message.
- [x] When an explicit codec's hardware encoder cannot take the frame, use that codec's software
      encoder with a warning.
- [x] Pass the encoded frame size from packed-alpha sidecar generation and from transcodes.

## Built

- `src/video/encoder_selection.rs` replaces the three copies of the selection rule.
  `select_encoder(requested, frame_size, caps)` is the pure rule over the `EncoderCaps` trait;
  `device_caps(no_hwaccel)` opens the VA-API device, or returns `None` with `--no-hwaccel` or
  without the `hwaccel` feature. So `no_hwaccel` reaches the pure function as `caps = None`
  rather than as its own parameter, unlike the seam's wording; behaviour is the same.
- `EncoderChoice` records, in `oversized`, each hardware encoder passed over because the frame
  exceeds its maximum. `select_encoder` logs a warning per entry for an explicit codec, and info
  lines for the default, naming the final encoder. Callers still pass `(Codec,
  HwAcceleratedEncoding)` tuples; the user chose to leave that as it is.
- `VaapiCapFinder` implements `EncoderCaps`. It picks the entrypoint FFmpeg prefers (`EncSlice`,
  then `EncSliceLP`) and reads `VAConfigAttribMaxPictureWidth`/`Height`. Neither reported, or the
  query failing, means unlimited; a single unreported dimension is unlimited on its own. On the
  user's machine the query reports 8192x4352 for AV1 and H.265 and 4096x4096 for H.264.
- Packed-alpha sidecars choose through `packed_alpha_encoder`, with `PackedAlphaLayout::packed_dimensions`.
  Transcodes pass `encoded_frame_size`: the target resolution when scaling, else the input's.
- `transcode` and `transcode_burn_osd` now probe the input and choose the encoder before naming
  the output file, because its extension depends on the hardware/software choice. So the probe
  runs before the output-exists and same-file checks; the user accepted the new error order.
  `output_video_file` takes the chosen codec and mode, and `OutputVideoFileError` loses its
  now-unused profile-error variant.
- The `--video-codec` help and its man page mention the frame-size condition.

### Review fixes

- Added explicit-codec tests for a frame within a reported limit and a frame too tall, and a test
  of the fallback reason's text.
- The default path now logs the encoder it ends up with whenever it skipped one, as sidecar
  generation has no other "using codec" line.

### Gaps found along the way

None beyond the review's judgement calls above.

## Comments

Found in ticket 04 (`.scratch/chroma-key-overlays/issues/04-packed-alpha-sidecars.md`, "Gaps
found along the way": "the default encoder is chosen by capability only"). Carried by
`.scratch/handoffs/2026-11-15-packed-alpha-sidecars.md` and later handoffs as an unwritten
candidate; written up 2026-11-16.

Where the choice is made today, all by capability only (`VaapiCapFinder::can_encode`,
`src/video/hw_accel/vaapi_cap_finder.rs`, which checks for an encode entrypoint):

- `video::default_video_codec` (`src/video.rs`): prefers VAAPI AV1, then H.265, else software
  H.265.
- `TranscodeConfig::resolve_codec` (`src/video.rs`): the same rule, plus an explicit codec uses
  hardware whenever the device can encode it.
- `TranscodeVideoArgs::video_codec` (`src/cli/transcode_video_args.rs`): mirrors
  `resolve_codec`.

None of them knows the frame size, so the size has to reach the choice.

Open questions for triage:

- Limit source: a fixed 4096x2160, or queried per device. VA-API reports
  `VAConfigAttribMaxPictureWidth` and `VAConfigAttribMaxPictureHeight` per profile and
  entrypoint; check whether `cros_libva` exposes them.
- Explicit codec too large for its hardware encoder: fail early with a clear error, or fall back
  to that codec's software encoder with a warning.
- Scope: packed-alpha sidecars only, or every encode (a plain transcode of a video above the
  hardware limit has the same problem).
- The three copies of the selection rule above: fix each, or merge them first.
- Is it worth doing: the user has not yet confirmed rendering overlays above 2048 px wide.
