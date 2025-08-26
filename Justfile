
test:
    cargo nextest run

build-appimage:
    (cd appimage_builder && cargo run --release)

nixos-build-appimage:
    (cd appimage_builder && nix-shell shell.nix --run 'cargo run --release')

build:
    cargo build --release

nixos-build:
    nix-shell shell.nix --run 'cargo build --release'
