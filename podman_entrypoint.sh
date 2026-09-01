#!/bin/sh
set -e

# On NixOS hosts, AppImage files are transparently intercepted by a host-wide
# binfmt_misc registration (appimage_type_1/appimage_type_2) that redirects
# execution to a NixOS store path (e.g. /run/binfmt/appimage_type_2). That
# registration is visible inside this container too (binfmt_misc is a single
# kernel-global table shared across mount namespaces by default), but the
# interpreter path it points to does not exist in this container's rootfs,
# so trying to run the downloaded appimagetool AppImage fails with
# "No such file or directory (os error 2)".
#
# Mounting a fresh binfmt_misc instance here creates an empty, container-private
# table that shadows the host's registrations, so the statically linked
# appimagetool AppImage runs as a plain ELF binary instead of being routed
# through the (missing) NixOS interpreter. Requires CAP_SYS_ADMIN, which
# podman_build already grants.
mount -t binfmt_misc none /proc/sys/fs/binfmt_misc 2>/dev/null || true

exec cargo run --release
