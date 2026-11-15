# HD FPV video tool

## What it is and what it can be used for

It is a Unix compatible command line tool intended to be used for various tasks related to modifying videos recorded with the
DJI and Walksnail Avatar FPV video systems (but also works with other video sources) and working with `.osd` files recorded onto the goggles.

It can for example among other things transcode videos, fix the audio recorded by DJI air units and burn the OSD recorded using FPV.WTF onto a video.

## How to use

### Available commands

#### display-osd-file-info

Displays information about the specified OSD file like the recorded OSD layout and font variant which should be used to render the OSD file.

#### generate-overlay-frames

Generates OSD overlay frames.
This command generates numbered OSD frame images from the specified OSD file and writes them into the specified output directory.

Use this command when you want to generate OSD frame images to check what the OSD looks like or when you want to manually burn the OSD onto a video.

#### generate-overlay-video

Generates an OSD overlay video. By default, this command generates an MP4 with the OSD frames rendered from the specified OSD file. Each frame stores the OSD colors and transparency side by side ("packed alpha"), so the sidecar can use the fast default encoders, including hardware ones, while keeping smooth OSD edges. The generated video can then be used to play an FPV video with OSD without burning it into the video using `play-video-with-osd`. Use `--codec vp8` or `--codec vp9` when transparent WebM compatibility output is required.

#### cut-video

Cuts a video file without transcoding by specifying the desired start and/or end timestamp.

#### fix-video-audio

Fixes a DJI Air Unit video's audio synchronization and/or volume

#### transcode-video

Transcodes a video file optionally burning OSD onto it. Also provides the option to fix the audio synchronization and/or volume at the same time as transcoding and also to hide things like dead pixels or dirt on the lens.

#### play-video-with-osd

Plays a video using the MPV video player with OSD by overlaying a transparent OSD video in real time. The transparent OSD video can be generated with the `generate-overlay-video` command.

#### help

Prints the CLI commands or help of the given subcommand(s)

### Command aliases

The commands can be a bit long to write. For convenience they are aliased to the concatenation of the first letter of each word.
For example the `generate-overlay-video` command can also be called with the `gov` command.

### OSD fonts

Rendering an OSD needs the OSD font, for both DJI and Walksnail recordings. Two font formats are read:

* **PNG fonts**, the format of msp-osd v0.12.0 and later and of Walksnail Avatar: one PNG 256 glyphs tall, with 1 to 4 pages of 256 glyphs side by side.
* **Bin fonts**, the WTF.FPV format of msp-osd before v0.12.0: one file per page of 256 glyphs, `font[_<ident>][_hd].bin` and `font[_<ident>][_hd]_2.bin`.

Fonts are looked up in the font directory, `~/.local/share/hd_fpv_video_tool/fonts` by default. The `DJI_OSD_FONTS_DIR` environment variable or the `--font-dir` or `--osd-font-dir` option, depending on the command, picks another directory. The files use msp-osd's names, so the fonts on your DJI goggles' SD card can be copied as they are. `_hd` fonts are used for HD (24×36) glyphs and fonts without it for SD (36×54) glyphs.

The font is picked from the flight controller firmware the recording names. For Betaflight with HD glyphs the files looked up are, in order:

1. `font_btfl_hd.png`, then the legacy alias `font_bf_hd.png`
2. `font_bf_hd.bin`
3. the generic `font_hd.png`
4. the generic `font_hd.bin`

The other firmware names are `inav` (INAV), `ardu` (ArduPilot) and `ultr` with the legacy alias `ultra` (KISS Ultra); their bin fonts use `inav`, `ardu` and `ultra`. Walksnail recordings name INAV and ArduPilot only; other recordings use the generic font. `--font-ident` or `--osd-font-ident` forces an ident, which is used as given (for example `quic` for Quicksilver): `font_<ident>[_hd].png`, then `font_<ident>[_hd].bin`, then the generic fonts; an empty ident loads the generic fonts only. The font file loaded is logged.

Walksnail font files are not looked up by their own names: rename `<name>_36.png` to `font_<ident>.png` and `<name>_24.png` to `font_<ident>_hd.png`. [hd_fpv_osd_font_tool](https://github.com/shellixyz/hd_fpv_osd_font_tool) converts fonts between formats.

* [Fonts from the WTF.FPV project](https://github.com/fpv-wtf/msp-osd/tree/main/fonts)
* [Latest ArduPilot fonts](https://github.com/ArduPilot/ardupilot/tree/master/libraries/AP_OSD/fonts/HDFonts)
* [Latest ArduCustom fonts](https://github.com/ArduCustom/ardupilot/tree/master_custom/libraries/AP_OSD/fonts/HDFonts/DJI)

### Example usage

For these examples we are assuming that:

* You have installed the fonts in the default directory so that they will be found automatically
* These files are present in the current directory:
  * DJIG0000.osd (the OSD file recorded by your goggles hacked with FPV.WTF)
  * DJIG0000.mp4 (video recorded by your goggles)
  * DJIU0000.mp4 (video recorded by your air unit if it has the capability)

#### Transcoding a video and burning the OSD onto it

`hd_fpv_video_tool transcode-video --osd DJIG0000.mp4`

Will automatically use the `DJIG0000.osd` file in the same directory as the video and automatically select a name for the output file: `DJIG0000_transcoded.mp4`. The OSD file can automatically be found if it is named with the same `DJIGXXXX` prefix as the video file or with the same name but with `.osd` extension. You can also specify the OSD file to use and the output file name manually. The default encoder is `libx265` so the output is encoded with the H.265 codec but the video encoder used can be selected with the `--video-encoder` option. The above command is equivalent to:

`hd_fpv_video_tool transcode-video --osd-file DJIG0000.osd DJIG0000.mp4 DJIG0000_transcoded.mp4`

If you want to burn the OSD onto a video coming from a DJI FPV air unit with audio you can do so while also fixing the audio synchronization and volume using this command:

`hd_fpv_video_tool transcode-video --fix-audio --osd DJIU0000.mp4`

Run `hd_fpv_video_tool transcode-video --help` or `hd_fpv_video_tool help transcode-video` for a list of all the options available for this command.

Pass `--osd-overlay-video` to generate a sidecar instead of burning the OSD into the transcoded video. It defaults to a packed-alpha MP4 sidecar; use `--osd-overlay-video-codec vp8` or `vp9` for transparent WebM compatibility output.

#### Generating an OSD overlay video and playing an unmodified video with OSD

First we need to generate the OSD overlay video:

`hd_fpv_video_tool generate-overlay-video --target-video-file DJIG0000.mp4 DJIG0000.osd`

This command will encode a packed-alpha OSD overlay video and write it into the `DJIG0000_osd.mp4` file. The original video `DJIG0000.mp4` will not be modified. It is only used to choose the right resolution and OSD scaling for the output video. We can then use the `play-video-with-osd` command to play the `DJIG0000.mp4` file with overlaid OSD:

`hd_fpv_video_tool play-video-with-osd DJIG0000.mp4`

See [overlay video quality validation](docs/overlay-video-quality.md) for the opt-in fixture
check, visual review criteria, and benchmark procedure.

## Installation

### Easiest way

The easiest way is to use the AppImage provided in the [latest release](https://github.com/shellixyz/hd_fpv_video_tool/releases/latest). It includes all the necessary dependencies. It can be put anywhere on your filesystem, just make it executable and run it. The only disadvantage of this method is that since the AppImage file contains all the dependencies it is fairly big.

Note that the generated AppImage files have only been tested on Fedora and Ubuntu and that on some distros like Ubuntu you may still need to install `libfuse` v2 for it to work without using the `--appimage-extract` option. To install `libfuse` on Debian and derivatives like Ubuntu use this command: `sudo apt-get install -y libfuse2`.

### Building from source

#### Build dependencies

* [rust tools/toolchain](https://www.rust-lang.org/tools/install)
* ffmpeg libs and headers
* pkg-config
* clang

##### On Fedora

You probably want to instead install ffmpeg from RPM fusion on Fedora which has support for more codecs so start by installing the RPM Fusion repository:

`sudo dnf install "https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm"`

then run

`sudo dnf install -y ffmpeg{,-devel} clang`

##### On Debian or Debian derivatives

`sudo apt-get install -y ffmpeg libav{format,util,filter,device}-dev clang pkg-config`

##### On macOS

Install rust and ffmpeg packages with brew:

`brew install rust ffmpeg`

Then you can continue building the tool with cargo.

#### Building

`cargo install --locked --git https://github.com/shellixyz/hd_fpv_video_tool.git hd_fpv_video_tool`

#### Run-time dependencies

* [ffmpeg](https://ffmpeg.org/) built with support for the video codecs you want to use and also VP8/VP9 for using the `generate-overlay-video` command
* [MPV](https://mpv.io/) video player if you want to use the `play-video-with-osd` command

##### Installing on Fedora

`sudo dnf install -y ffmpeg-free mpv`

Note that you probably want to instead install ffmpeg from RPM fusion on Fedora which has support for more codecs:

```shell
sudo dnf install "https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm"
sudo dnf install -y ffmpeg
```

##### Installing on Debian or Debian derivatives

`sudo apt-get install -y ffmpeg mpv`
