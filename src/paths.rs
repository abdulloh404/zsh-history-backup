use anyhow::{Context, Result, bail};
use std::env;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub struct AppPaths {
    pub history_file: PathBuf,
    pub state_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub exports_dir: PathBuf,
    pub log_file: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let home_dir = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .context("HOME is not set")?;
        if !home_dir.is_absolute() {
            bail!("HOME must be an absolute path");
        }

        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home_dir.join(".local/state"));
        let state_dir = state_home.join("zsh-history-backup");

        Ok(Self {
            history_file: home_dir.join(".zsh_history"),
            state_dir: state_dir.clone(),
            backups_dir: state_dir.join("backups"),
            exports_dir: state_dir.join("exports"),
            log_file: state_dir.join("logs/backup.log"),
        })
    }

    pub fn ensure_layout(&self) -> Result<()> {
        ensure_private_directory(&self.state_dir)?;
        ensure_private_directory(&self.backups_dir)?;
        ensure_private_directory(&self.exports_dir)?;
        let logs_dir = self
            .log_file
            .parent()
            .context("log path has no parent directory")?;
        ensure_private_directory(logs_dir)
    }
}

fn ensure_private_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory {}", path.display()))?;
    if !path
        .metadata()
        .with_context(|| format!("failed to inspect directory {}", path.display()))?
        .is_dir()
    {
        bail!("{} is not a directory", path.display());
    }
    fs::set_permissions(path, Permissions::from_mode(0o700))
        .with_context(|| format!("failed to secure directory {}", path.display()))
}
