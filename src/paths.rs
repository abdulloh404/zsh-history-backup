use crate::config::AppConfig;
use anyhow::{Context, Result, bail};
use std::env;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

pub struct AppPaths {
    pub history_file: PathBuf,
    pub state_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub exports_dir: PathBuf,
    pub log_file: PathBuf,
}

impl AppPaths {
    pub fn discover(config_override: Option<&Path>) -> Result<Self> {
        let home_dir = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .context("HOME is not set")?;
        if !home_dir.is_absolute() {
            bail!("HOME must be an absolute path");
        }

        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home_dir.join(".config"));
        let config_file = match config_override {
            Some(path) => expand_tilde(path, &home_dir),
            None => config_home.join("zsh-history-backup/config.toml"),
        };
        let config = AppConfig::load(&config_file, config_override.is_some())?;

        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home_dir.join(".local/state"));
        let state_dir = state_home.join("zsh-history-backup");
        let history_file = match config.history_file {
            Some(path) => resolve_configured_path(&path, &home_dir, "history_file")?,
            None => home_dir.join(".zsh_history"),
        };
        let backups_dir = match config.backup_dir {
            Some(path) => resolve_backup_dir(&path, &home_dir, &state_dir)?,
            None => state_dir.join("backups"),
        };

        Ok(Self {
            history_file,
            state_dir: state_dir.clone(),
            backups_dir,
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

fn expand_tilde(path: &Path, home_dir: &Path) -> PathBuf {
    path.strip_prefix("~")
        .map(|relative| home_dir.join(relative))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_configured_path(path: &Path, home_dir: &Path, field: &str) -> Result<PathBuf> {
    let resolved = expand_tilde(path, home_dir);
    if !resolved.is_absolute() {
        bail!("config field {field} must be an absolute path or start with ~/");
    }
    Ok(resolved)
}

fn resolve_backup_dir(path: &Path, home_dir: &Path, state_dir: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || path.components().any(|part| part == Component::ParentDir) {
        bail!("config field backup_dir cannot be empty or contain ..");
    }

    let resolved = if path.is_absolute() || path.starts_with("~") {
        resolve_configured_path(path, home_dir, "backup_dir")?
    } else {
        state_dir.join(path)
    };

    if !resolved.starts_with(state_dir) {
        bail!(
            "config field backup_dir must stay inside {}; use a relative path such as backups",
            state_dir.display()
        );
    }

    Ok(resolved)
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
