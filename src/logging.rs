use anyhow::{Context, Result};
use chrono::{Local, SecondsFormat};
use std::fs::{OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub fn append(path: &Path, level: &str, operation: &str, message: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("failed to open log {}", path.display()))?;
    file.set_permissions(Permissions::from_mode(0o600))
        .with_context(|| format!("failed to secure log {}", path.display()))?;

    let timestamp = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
    writeln!(file, "{timestamp}\t{level}\t{operation}\t{message}")
        .with_context(|| format!("failed to append log {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush log {}", path.display()))
}
