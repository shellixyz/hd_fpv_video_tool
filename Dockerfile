# The AppImage only runs on systems with a glibc at least as recent as the one of the build system, so it is
# built on an old distribution. Ubuntu 22.04 is what the AppImage catalog (appimage.github.io) tests with.
FROM ubuntu:22.04
WORKDIR /hd_fpv_video_tool/appimage_builder

ARG DEBIAN_FRONTEND=noninteractive

# FFmpeg shared build targeting glibc 2.28 (https://github.com/BtbN/FFmpeg-Builds), much more recent than the one of
# the distribution
ARG FFMPEG_BUILD_URL=https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.1-latest-linux64-gpl-shared-8.1.tar.xz

# libva from the distribution is too old and only looks for drivers where the distribution installs them. The X11
# and Wayland backends are built too so that they match the libva version (they are used by mpv).
ARG LIBVA_VERSION=2.24.1
ARG LIBVA_DRIVERS_PATH=/usr/lib/x86_64-linux-gnu/dri:/usr/lib64/dri:/usr/lib/dri:/run/opengl-driver/lib/dri

RUN apt-get update && apt-get install --no-install-recommends -y \
	build-essential ca-certificates clang curl desktop-file-utils file git libdrm-dev libfuse2 libwayland-dev \
	libx11-dev libx11-xcb-dev libxcb-dri3-dev libxext-dev libxfixes-dev meson mpv ninja-build pkg-config xz-utils

RUN curl -sSfL "https://github.com/intel/libva/archive/refs/tags/${LIBVA_VERSION}.tar.gz" | tar xz -C /tmp \
	&& cd "/tmp/libva-${LIBVA_VERSION}" \
	&& meson setup build --prefix=/usr/local --libdir=lib --buildtype=release \
		-Ddriverdir="${LIBVA_DRIVERS_PATH}" -Dwith_x11=yes -Dwith_glx=no -Dwith_wayland=yes \
	&& ninja -C build install \
	&& cd / && rm -rf "/tmp/libva-${LIBVA_VERSION}"

RUN mkdir /opt/ffmpeg \
	&& curl -sSfL "${FFMPEG_BUILD_URL}" | tar xJ -C /opt/ffmpeg --strip-components=1 \
	&& echo /opt/ffmpeg/lib > /etc/ld.so.conf.d/ffmpeg.conf \
	&& ldconfig
ENV FFMPEG_DIR=/opt/ffmpeg
ENV PATH=/opt/ffmpeg/bin:$PATH

ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable

COPY podman_entrypoint.sh /usr/local/bin/podman_entrypoint.sh
RUN chmod +x /usr/local/bin/podman_entrypoint.sh

ENTRYPOINT ["/usr/local/bin/podman_entrypoint.sh"]
# CMD ["bash"]
