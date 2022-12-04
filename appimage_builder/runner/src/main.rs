
use std::{env, env::current_exe, path::PathBuf};
use anyhow::anyhow;


fn main() -> anyhow::Result<()> {

    let appimage_exe = current_exe()?;
    let appimage_dir = appimage_exe.parent().ok_or_else(|| anyhow!("exe has no parent"))?;

    // set LD_LIBRARY_PATH to own lib dir
    let lib_dir = appimage_dir.join("lib64");
    env::set_var("LD_LIBRARY_PATH", lib_dir);

    // install own bin dir in front of PATH
    let paths = match env::var_os("PATH") {
        Some(value) => env::split_paths(&value).collect::<Vec<PathBuf>>(),
        None => vec![],
    };

    let new_paths = env::join_paths(
        [vec![appimage_dir.join("bin")], paths].into_iter().flatten().collect::<Vec<PathBuf>>()
    )?;

    env::set_var("PATH", new_paths);

    // exec app
    let application_bin_path = appimage_dir.join("bin/bin");
    Err(exec::execvp(application_bin_path, std::env::args()))?;

    Ok(())
}