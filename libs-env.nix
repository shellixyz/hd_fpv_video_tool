let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
        libva
        openssl
        clang
        libclang
        ffmpeg
    ];
    nativeBuildInputs = with pkgs; [
        pkg-config
        rustPlatform.bindgenHook
    ];
  }
