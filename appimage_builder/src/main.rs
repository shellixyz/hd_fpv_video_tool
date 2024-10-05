use std::{io::{self, Write}, path::{Path, PathBuf}, process::Command, env::set_current_dir, fs::{File, self}, os::unix::fs::PermissionsExt};

use anyhow::{anyhow, Context};
use env_logger::fmt::Color;
use futures_util::stream::StreamExt;
use indicatif::{ProgressStyle, ProgressBar};
use regex::Regex;
use indoc::indoc;
use which::which;

#[cfg(not(target_os = "linux"))]
compile_error!("this program is only intended to be run on linux");

const APPIMAGETOOL_BIN_NAME: &str = "appimagetool";

const APPIMAGETOOL_URL: &str = "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage";

const DEP_BINARIES: [&str; 2] = [
    "ffmpeg",
    "mpv",
];

const EXCLUDE_LIBS: [&str; 53] = [
    "libasound", "libcdio_paranoia", "libcdio_cdda", "libcdio", "libm", "libdrm", "libEGL", "libgbm", "libwayland-egl", "libwayland-client", "libGL", "libjack",
    "liblcms2", "libarchive", "libpulse", "libsamplerate", "libuchardet", "libvulkan", "libwayland-cursor", "libxkbcommon", "libX11", "libXss", "libXext", "libXinerama",
    "libXrandr", "libXv", "libz", "libgcc_s", "libc", "libGLdispatch", "libwayland-server", "libexpat", "libstdc++", "libffi", "libGLX", "libacl", "liblzma", "libzstd",
    "liblz4", "libxml2", "libdbus-1", "libxcb", "libXrender", "libsndfile", "libsystemd", "libasyncns", "libXau", "libFLAC", "libvorbis", "libvorbisenc", "libopus", "libogg", "libcap"
];

const RUNNER_BIN_PATH: &str = "target/release/appimage_runner";

fn create_path<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    std::fs::create_dir_all(&path).map_err(|error|
        anyhow!("failed to create dir `{}`: {error}", path.as_ref().to_string_lossy())
    )
}

fn binary_linked_libs<P: AsRef<Path>>(bin_path: P) -> anyhow::Result<Vec<PathBuf>> {
    let ldd_output = Command::new("ldd").arg(bin_path.as_ref()).output()?;
    if ! ldd_output.status.success() {
        return Err(anyhow!("command failed ({}): ldd {}: {}", ldd_output.status, bin_path.as_ref().to_string_lossy(), String::from_utf8_lossy(&ldd_output.stderr)));
    }
    let lib_re = Regex::new("=> (.+) \\(").unwrap();
    let ldd_output = std::str::from_utf8(&ldd_output.stdout)?;
    Ok(lib_re.captures_iter(ldd_output).map(|captures|
        PathBuf::from(captures.get(1).unwrap().as_str().to_string())
    ).collect())
}

fn install_binary_shared_libs<P: AsRef<Path>, Q: AsRef<Path>>(binary_path: P, lib_dir_path: Q) -> anyhow::Result<()> {
    create_path(&lib_dir_path)?;
    for lib_path in binary_linked_libs(&binary_path)? {
        let lib_file_name = lib_path.file_name().unwrap().to_str().unwrap();
        if EXCLUDE_LIBS.iter().any(|ex_name| lib_file_name.starts_with(&format!("{ex_name}."))) { continue; }
        let to_path =  lib_dir_path.as_ref().join(lib_path.file_name().unwrap());
        log::debug!("copying `{}` => `{}`", lib_path.to_string_lossy(), to_path.to_string_lossy());
        std::fs::copy(&lib_path, &to_path)
            .with_context(|| format!("{} linked libs copy: failed copying `{}` => `{}`",
                binary_path.as_ref().to_string_lossy(),
                lib_path.to_string_lossy(),
                to_path.to_string_lossy()
            ))?;
    }
    Ok(())
}

fn build_application_binary(application_name: &str) -> anyhow::Result<()> {

    log::info!("building binary: {application_name}");
    println!();

    let build_status = Command::new("cargo")
        .args(["build", "--bin", application_name, "--release"])
        .current_dir("..")
        .status()
        .map_err(|error| anyhow!("failed to launch cargo: {error}"))?;

    if ! build_status.success() {
        println!();
        return Err(anyhow!("failed to build binary: cargo: {build_status}"));
    }

    println!();
    Ok(())
}

fn build_runner() -> anyhow::Result<()> {

    log::info!("building runner");
    println!();

    let build_status = Command::new("cargo")
        .args(["build", "--bin", "appimage_runner", "--release"])
        .current_dir("runner")
        .status()
        .map_err(|error| anyhow!("failed to launch cargo: {error}"))?;

    if ! build_status.success() {
        println!();
        return Err(anyhow!("failed to build runner: cargo: {build_status}"));
    }

    println!();
    Ok(())
}

fn setup_logger() {
    env_logger::builder()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level());
            write!(buf, "{:<5}", level_style.value(record.level()))?;
            let mut style = buf.style();
            style.set_color(Color::White).set_bold(true);
            write!(buf, "{}", style.value(" > "))?;
            writeln!(buf, "{}", record.args())
        })
        .parse_filters("info")
        .init();
    println!();
}

fn install_runner<P: AsRef<Path>>(appdir_path: P) -> anyhow::Result<()> {
    log::info!("installing runner");
    let runner_dest_path = appdir_path.as_ref().join("AppRun");
    std::fs::copy(RUNNER_BIN_PATH, &runner_dest_path)
        .with_context(|| format!("failed to install runner at {}", runner_dest_path.to_string_lossy()))?;
    Ok(())
}

fn install_binary_dependency<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(binary_path: P, bin_dir_path: Q, lib_dir_path: R) -> anyhow::Result<()> {
    let binary_path_str = binary_path.as_ref().to_string_lossy();
    if ! binary_path.as_ref().exists() { return Err(anyhow!("binary dependency not found: {binary_path_str}")) }
    log::info!("installing binary dependency: {binary_path_str}");
    let bin_dest_path = bin_dir_path.as_ref().join(binary_path.as_ref().file_name().unwrap());
    std::fs::copy(&binary_path, &bin_dest_path)
        .with_context(|| format!("failed to install binary dependency at {}", bin_dest_path.to_string_lossy()))?;
    log::info!("installing shared libs for binary: {binary_path_str}");
    install_binary_shared_libs(&binary_path, lib_dir_path)?;
    Ok(())
}

fn install_application_binary<P: AsRef<Path>, Q: AsRef<Path>>(binary_path: P, bin_dir_path: Q) -> anyhow::Result<()> {
    log::info!("installing application binary");
    let binary_dest_path = bin_dir_path.as_ref().join("bin");
    std::fs::copy(binary_path, &binary_dest_path)
        .with_context(|| format!("failed to install application binary at {}", binary_dest_path.to_string_lossy()))?;
    Ok(())
}

fn install_desktop_file<P: AsRef<Path>>(appdir_path: P, application_name: &str, application_version: &str) -> anyhow::Result<()> {
    log::info!("installing desktop file");
    let desktop_file_path = appdir_path.as_ref().join(format!("{application_name}.desktop"));
    let mut file = std::fs::File::create(&desktop_file_path)
        .with_context(|| format!("failed to create desktop file: {}", desktop_file_path.to_string_lossy()))?;
    file.write_all("[Desktop Entry]\n".as_bytes())?;
    write!(file, "Name={application_name}")?;
    file.write_all(indoc!{"
        Exec=bin
        Icon=icon
        Type=Application
        Categories=Utility
    "}.as_bytes())?;
    write!(file, "X-AppImage-Version={application_version}")?;
    Ok(())
}

fn install_icon_file<P: AsRef<Path>>(appdir_path: P) -> anyhow::Result<()> {
    log::info!("installing icon file");
    let icon_file_path = appdir_path.as_ref().join("icon.png");
    std::fs::write(&icon_file_path, [])
        .with_context(|| format!("failed to icon file: {}", icon_file_path.to_string_lossy()))?;
    Ok(())
}

async fn download_file_with_progress(url: &str, dest_path: &str) -> anyhow::Result<()> {
    let response = reqwest::get(url).await?;

    let status_code = response.status();
    if ! status_code.is_success() {
        return Err(anyhow!("failed to download: {}", status_code));
    }

    let total_size = response.content_length().unwrap_or(0);

    let mut dest_file = File::create(dest_path)?;
    let mut downloaded = 0;
    let progress_style = ProgressStyle::with_template("{wide_bar} {percent:>3}% [ETA {eta:>3}]").unwrap();
    let progress_bar = ProgressBar::new(total_size).with_style(progress_style);

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        dest_file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        progress_bar.set_position(downloaded);
    }

    Ok(())
}

async fn prepare_appimagetool() -> anyhow::Result<PathBuf> {
    if let Ok(appimagetool_path) = which(APPIMAGETOOL_BIN_NAME) {
        log::info!("AppImage tool found: {}", appimagetool_path.to_string_lossy());
        return Ok(appimagetool_path);
    }

    let appimagetool_path = Path::new(APPIMAGETOOL_BIN_NAME);

    if ! appimagetool_path.exists() {
        log::info!("AppImage tool not found, downloading");
        download_file_with_progress(APPIMAGETOOL_URL, APPIMAGETOOL_BIN_NAME).await.context("appimagetool")?;
    }

    if ! appimagetool_path.is_file() { log::error!("{APPIMAGETOOL_BIN_NAME} exists but is not a regular file"); }

    std::fs::set_permissions(appimagetool_path, fs::Permissions::from_mode(0o755)).context(format!("failed to set {APPIMAGETOOL_BIN_NAME} permissions"))?;

    Ok([Path::new("."), appimagetool_path].iter().collect())
}

fn generate_appimage<P: AsRef<Path>, Q: AsRef<Path>, R: AsRef<Path>>(appimagetool_bin_path: P, appimage_path: Q, appdir_path: R) -> anyhow::Result<()> {

    let appimage_path = appimage_path.as_ref();

    log::info!("generating AppImage image: {}", appimage_path.to_string_lossy());

    let appimagetool_output = Command::new(appimagetool_bin_path.as_ref())
        .args([appdir_path.as_ref(), appimage_path])
        .output()
        .map_err(|error| anyhow!("failed to launch {APPIMAGETOOL_BIN_NAME}: {error}"))?;

    if ! appimagetool_output.status.success() {
        log::error!("failed to generate AppImage image: {APPIMAGETOOL_BIN_NAME}: {}", appimagetool_output.status);
        println!();
        io::stderr().write_all(&appimagetool_output.stderr).unwrap();
        return Err(anyhow!("failed to generate AppImage image: {APPIMAGETOOL_BIN_NAME}: {}", appimagetool_output.status));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    setup_logger();

    let toml = cargo_toml::Manifest::from_path("../Cargo.toml")?;
    let application_name = toml.package().name();
    let application_version = toml.package().version();
    let application_binary_path = Path::new("target/release").join(application_name);
    let appdir_path = Path::new("target").join(application_name).with_extension("AppDir");
    let lib_dir_path = appdir_path.join("lib64");
    let bin_dir_path = appdir_path.join("bin");

    build_application_binary(application_name)?;
    build_runner()?;

    set_current_dir("..").context("failed to change current dir")?;

    log::info!("creating app dir: {}", appdir_path.to_string_lossy());
    create_path(&appdir_path)?;

    log::info!("creating app lib dir: {}", lib_dir_path.to_string_lossy());
    create_path(&lib_dir_path)?;

    log::info!("creating app bin dir: {}", bin_dir_path.to_string_lossy());
    create_path(&bin_dir_path)?;

    install_desktop_file(&appdir_path, application_name, application_version)?;
    install_icon_file(&appdir_path)?;
    install_runner(&appdir_path)?;
    install_application_binary(application_binary_path, &bin_dir_path)?;

    for binary_path in DEP_BINARIES {
        let Ok(binary_path) = which(binary_path) else {
            let err_msg = format!("binary dependency not found: {binary_path}");
            log::error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        };
        install_binary_dependency(binary_path, &bin_dir_path, &lib_dir_path)?;
    }

    let appimage_path = Path::new(application_name).with_extension("AppImage");
    let appimagetool_path = prepare_appimagetool().await?;
    generate_appimage(appimagetool_path, appimage_path, &appdir_path)?;

    Ok(())
}