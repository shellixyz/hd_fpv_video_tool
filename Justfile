
test:
    cargo nextest run

build-appimage:
    (cd appimage_builder && cargo run --release)

build:
    cargo build --release