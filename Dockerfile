FROM rust
WORKDIR /hd_fpv_video_tool/appimage_builder

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install --no-install-recommends -y desktop-file-utils ffmpeg libfuse2 libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev mpv clang pkg-config

ENTRYPOINT ["cargo", "run", "--release"]
# CMD ["bash"]