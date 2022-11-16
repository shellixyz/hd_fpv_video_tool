
use std::{
    fmt::Display,
    path::{
        PathBuf,
        Path,
    },
    io::Error as IOError,
};

use derive_more::Error;


#[derive(Debug, Error)]
pub struct CreatePathError {
    path: PathBuf,
    error: IOError,
}

impl CreatePathError {
    pub fn new<P: AsRef<Path>>(path: P, error: IOError) -> Self {
        Self { path: path.as_ref().to_path_buf(), error }
    }
}

impl Display for CreatePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to create path {}: {}", self.path.to_string_lossy(), self.error)
    }
}

pub fn create_path<P: AsRef<Path>>(path: P) -> Result<(), CreatePathError> {
    std::fs::create_dir_all(&path).map_err(|error| CreatePathError::new(&path, error) )
}