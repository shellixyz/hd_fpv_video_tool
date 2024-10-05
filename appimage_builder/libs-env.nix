let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
        clang
	libclang
	openssl
	ffmpeg_7
	mpv-unwrapped
	appimagekit
    ];
    nativeBuildInputs = with pkgs; [
        pkg-config
	rustPlatform.bindgenHook
    ];
  }
