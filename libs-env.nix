let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
        clang
	libclang
        ffmpeg_7
    ];
    nativeBuildInputs = with pkgs; [
        pkg-config
	rustPlatform.bindgenHook
    ];
  }
