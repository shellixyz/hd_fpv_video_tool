# 05: Fall back to software encoding when the frame exceeds the hardware encoder's limits

**What to build:** when the default encoder would be a VAAPI hardware encoder but the frames to
encode are larger than it supports, encode in software (libx265) and log why, instead of letting
FFmpeg fail partway through. The case that motivates it is a packed-alpha sidecar: its frame is
`align64(W) + W` wide (`PackedAlphaLayout`, `src/osd/overlay.rs`), so an overlay above about
2048 px wide, as when scaling to a 4K target, exceeds the 4096-wide limit common to hardware
encoders.

**Status:** needs-triage

- [ ] Find the hardware encoder's maximum frame size for the chosen codec.
- [ ] When choosing the default encoder, skip hardware codecs whose maximum the frame exceeds, and
      fall back to software H.265 with a log message.
- [ ] Decide and implement what happens when the user explicitly requests a codec whose hardware
      encoder cannot take the frame.

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
