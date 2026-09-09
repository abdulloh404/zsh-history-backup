use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub history_file: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
}

impl AppConfig {
    pub fn load(path: &Path, required: bool) -> Result<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound && !required => {
                return Ok(Self::default());
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read config {}", path.display()));
            }
        };

        toml::from_str(&contents)
            .with_context(|| format!("failed to parse config {}", path.display()))
    }
}
