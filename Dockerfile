FROM rust
WORKDIR /hd_fpv_video_tool/appimage_builder

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install --no-install-recommends -y desktop-file-utils ffmpeg libfuse2 libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev libva-dev mpv clang pkg-config

COPY podman_entrypoint.sh /usr/local/bin/podman_entrypoint.sh
RUN chmod +x /usr/local/bin/podman_entrypoint.sh

ENTRYPOINT ["/usr/local/bin/podman_entrypoint.sh"]
# CMD ["bash"]