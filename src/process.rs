
use std::{ffi::OsStr, process, fmt::Display};

use derive_more::{Deref, DerefMut};


#[derive(Deref, DerefMut)]
pub struct Command(process::Command);

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Self(process::Command::new(program))
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let components = [
                vec![self.get_program().to_string_lossy()],
                self.get_args().map(OsStr::to_string_lossy).collect::<Vec<_>>()
            ]
            .iter()
            .flatten()
            .map(|comp| {
                if comp.contains(' ') {
                    format!("\"{comp}\"")
                } else {
                    comp.to_string()
                }
            })
            .collect::<Vec<_>>();
        f.write_str(components.join(" ").as_str())
    }
}
