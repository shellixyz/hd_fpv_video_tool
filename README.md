# DJI FPV video tool

## What it is and what it can be used for

It is a unix compatible command line tool intended to be used for various tasks related to modifying videos recorded with the
DJI FPV video system (but also works with other video sources) and working with `.osd` files recorded by DJI goggles hacked thanks
to the [FPV.WTF](https://github.com/fpv-wtf) project. This project is not affiliated to the DJI company in any way.

## How to use

### Available commands

#### display-osd-file-info

Displays information about the specified OSD file like the recorded OSD layout and font variant which should be used to render the OSD file.

#### generate-overlay-frames

Generates OSD overlay frames.
This command generates numbered OSD frame images from the specified WTF.FPV OSD file and writes them into the specified output directory.

Use this command when you want to generate OSD frame images to check what the OSD looks like or when you want to manually burn the OSD onto a video.

#### generate-overlay-video

Generates an OSD overlay video. This command generates a transparent video with the OSD frames rendered from the specified WTF.FPV OSD file.  The generated video can then be used to play an FPV video with OSD without having to burn the OSD into the video using the `play-video-with-osd` command (or any other video player which can overlay a VP8/9 transparent video other another video in real time).

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

To generate OSD overlays the OSD fonts are needed. The same OSD font files you are using on your goggles can be used. You can put the files inside the `~/.local/share/hd_fpv_video_tool/fonts` directory so that the program will use them automatically. You can also put them in any location on your filesystem and tell the program where to look using the `DJI_OSD_FONTS_DIR` environment variable or using the `--font-dir` or `--osd-font-dir` options depending on the command.

### Example usage

For these examples we are assuming that:
- You have installed the fonts in the default directory so that they will be found automatically
- These files are present in the current directory:
    - DJIG0000.osd (the OSD file recorded by your goggles hacked with FPV.WTF)
    - DJIG0000.mp4 (video recorded by your goggles)
    - DJIU0000.mp4 (video recorded by your air unit if it has the capability)

#### Transcoding a video and burning the OSD onto it

`hd_fpv_video_tool transcode-video --osd DJIG0000.mp4`

Will automatically use the `DJIG0000.osd` file in the same directory as the video and automatically select a name for the output file: `DJIG0000_transcoded.mp4`. The OSD file can automatically be found if it is named with the same `DJIGXXXX` prefix as the video file or with the same name but with `.osd` extension. You can also specify the OSD file to use and the output file name manually. The default encoder is `libx265` so the output is encoded with the H.265 codec but the video encoder used can be selected with the `--video-encoder` option. The above command is equivalent to:

`hd_fpv_video_tool transcode-video --osd-file DJIG0000.osd DJIG0000.mp4 DJIG0000_transcoded.mp4`

If you want to burn the OSD onto a video coming from a DJI FPV air unit with video you can do so while also fixing the audio synchronization and volume using this command:

`hd_fpv_video_tool transcode-video --fix-audio --osd DJIU0000.mp4`

Run `hd_fpv_video_tool transcode-video --help` or `hd_fpv_video_tool help transcode-video` for a list of all the options available for this command.

#### Generating a transparent OSD overlay video and playing an unmodified video with OSD

First we need to generate the transparent OSD overlay video:

`hd_fpv_video_tool generate-overlay-video --target-video-file DJIG0000.mp4 DJIG0000.osd`

This command will encode a transparent OSD overlay video encoded with the VP8 coded by default and write it into the `DJIG0000_osd.webm` file. The original video `DJIG0000.mp4` will not be modified. It is only used to choose the right resolution and OSD scaling for the output video. We can then use the `play-video-with-osd` command to play the `DJIG0000.mp4` file with overlayed OSD:

`hd_fpv_video_tool play-video-with-osd DJIG0000.mp4`

## Installation

### Easiest way

The easiest way is to use the AppImage provided in the [latest release](https://github.com/shellixyz/hd_fpv_video_tool/releases/latest). It includes all the necessary dependencies. It can be put anywhere on your filesystem, just make it executable and run it. The only disadvantage of this method is that since the AppImage file contains all the dependencies so it is fairly big.

Note that the generated AppImage files have only been tested on Fedora and Ubuntu and that on you may still need to install `libfuse` v2 for it to work without using the `--appimage-extract` option. To install `libfuse` on Debian and derivatives like Ubuntu use this command: `sudo apt-get install -y libfuse2`.

### Building from source

#### Build dependencies

- [rust tools/toolchain](https://www.rust-lang.org/tools/install)
- ffmpeg libs and headers
- pkg-config
- clang

##### On Fedora

You probably want to instead install ffmpeg from RPM fusion on Fedora which has support for more codecs so start by installing the RPM Fusion repository:

`sudo dnf install "https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm"`

then run

`sudo dnf install -y ffmpeg{,-devel} clang`

##### On Debian or Debian derivatives

`sudo apt-get install -y ffmpeg libav{format,util,filter,device}-dev clang pkg-config`

##### On MacOSX

It should work on MacOSX but I do not have access to a machine with MacOSX to test

#### Building

`cargo install --locked https://github.com/shellixyz/hd_fpv_video_tool.git hd_fpv_video_tool`

#### Run-time dependencies

- [ffmpeg](https://ffmpeg.org/) built with support for the video codecs you want to use and also VP8/VP9 for using the `generate-overlay-video` command
- [MPV](https://mpv.io/) video player if you want to use the `play-video-with-osd` command

##### Installing on Fedora

`sudo dnf install -y ffmpeg-free mpv`

Note that you probably want to instead install ffmpeg from RPM fusion on Fedora which has support for more codecs:

```
sudo dnf install "https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm"
sudo dnf install -y ffmpeg
```

##### Installing on Debian or Debian derivatives

`sudo apt-get install -y ffmpeg mpv`
