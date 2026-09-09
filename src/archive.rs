use crate::paths::AppPaths;
use anyhow::{Context, Result, bail};
use chrono::{Local, NaiveDate, TimeZone};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::{Builder, NamedTempFile};

pub enum ArchiveReport {
    Backup {
        output: PathBuf,
        source_bytes: u64,
    },
    Export {
        output: PathBuf,
        target_date: NaiveDate,
        selected_entries: u64,
        timestamped_entries: u64,
        source_bytes: u64,
    },
}

impl ArchiveReport {
    pub fn output_path(&self) -> &Path {
        match self {
            Self::Backup { output, .. } | Self::Export { output, .. } => output,
        }
    }

    pub fn log_message(&self, source: &Path) -> String {
        match self {
            Self::Backup {
                output,
                source_bytes,
            } => format!(
                "source={} output={} source_bytes={source_bytes}",
                source.display(),
                output.display()
            ),
            Self::Export {
                output,
                target_date,
                selected_entries,
                timestamped_entries,
                source_bytes,
            } => format!(
                "source={} output={} date={target_date} selected_entries={selected_entries} timestamped_entries={timestamped_entries} source_bytes={source_bytes}",
                source.display(),
                output.display()
            ),
        }
    }
}

pub fn backup(paths: &AppPaths, name: Option<&str>) -> Result<ArchiveReport> {
    let (source, source_bytes) = open_source(&paths.history_file)?;
    let temp = new_private_temp(&paths.backups_dir)?;
    let mut encoder = GzEncoder::new(temp, Compression::default());
    let mut reader = BufReader::new(source.take(source_bytes));
    let copied = std::io::copy(&mut reader, &mut encoder)
        .with_context(|| format!("failed to read {}", paths.history_file.display()))?;

    if copied != source_bytes {
        bail!(
            "{} changed while it was being read; no backup was published (expected {source_bytes} bytes, read {copied})",
            paths.history_file.display()
        );
    }

    let temp = finish_gzip(encoder)?;
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let stem = match name {
        Some(name) => format!("{timestamp}_{name}"),
        None => format!("{timestamp}_full"),
    };
    let output = persist_unique(temp, &paths.backups_dir, &stem)?;

    Ok(ArchiveReport::Backup {
        output,
        source_bytes,
    })
}

pub fn export_date(
    paths: &AppPaths,
    target_date: NaiveDate,
    name: Option<&str>,
) -> Result<ArchiveReport> {
    let (source, source_bytes) = open_source(&paths.history_file)?;
    let temp = new_private_temp(&paths.exports_dir)?;
    let mut encoder = GzEncoder::new(temp, Compression::default());
    let mut reader = BufReader::new(source.take(source_bytes));
    let mut line = Vec::new();
    let mut source_bytes_read = 0_u64;
    let mut timestamped_entries = 0_u64;
    let mut selected_entries = 0_u64;
    let mut include_current_entry = false;

    loop {
        line.clear();
        let bytes_read = reader
            .read_until(b'\n', &mut line)
            .with_context(|| format!("failed to read {}", paths.history_file.display()))?;
        if bytes_read == 0 {
            break;
        }
        source_bytes_read += bytes_read as u64;

        if let Some(timestamp) = parse_extended_history_timestamp(&line) {
            timestamped_entries += 1;
            include_current_entry = Local
                .timestamp_opt(timestamp, 0)
                .single()
                .is_some_and(|date_time| date_time.date_naive() == target_date);
            if include_current_entry {
                selected_entries += 1;
            }
        }

        if include_current_entry {
            encoder
                .write_all(&line)
                .context("failed to write compressed export")?;
        }
    }

    if source_bytes_read != source_bytes {
        bail!(
            "{} changed while it was being read; no export was published (expected {source_bytes} bytes, read {source_bytes_read})",
            paths.history_file.display()
        );
    }
    if timestamped_entries == 0 {
        bail!(
            "no EXTENDED_HISTORY timestamps were found in {}; no export was published",
            paths.history_file.display()
        );
    }

    let temp = finish_gzip(encoder)?;
    let stem = match name {
        Some(name) => format!("{target_date}_{name}"),
        None => format!("{target_date}_history"),
    };
    let output = persist_unique(temp, &paths.exports_dir, &stem)?;

    Ok(ArchiveReport::Export {
        output,
        target_date,
        selected_entries,
        timestamped_entries,
        source_bytes,
    })
}

fn open_source(path: &Path) -> Result<(File, u64)> {
    let source = OpenOptions::new()
        .read(true)
        .open(path)
        .with_context(|| format!("failed to open {} for reading", path.display()))?;
    let metadata = source
        .metadata()
        .with_context(|| format!("failed to inspect {}", path.display()))?;
    if !metadata.is_file() {
        bail!("{} is not a regular file", path.display());
    }
    Ok((source, metadata.len()))
}

fn new_private_temp(directory: &Path) -> Result<NamedTempFile> {
    Builder::new()
        .prefix(".zsh-history-backup-")
        .tempfile_in(directory)
        .with_context(|| {
            format!(
                "failed to create a temporary file in {}",
                directory.display()
            )
        })
}

fn finish_gzip(encoder: GzEncoder<NamedTempFile>) -> Result<NamedTempFile> {
    let temp = encoder.finish().context("failed to finish gzip archive")?;
    temp.as_file()
        .sync_all()
        .context("failed to sync gzip archive")?;
    Ok(temp)
}

fn persist_unique(mut temp: NamedTempFile, directory: &Path, stem: &str) -> Result<PathBuf> {
    for sequence in 0_u32..10_000 {
        let file_name = if sequence == 0 {
            format!("{stem}.zsh-history.gz")
        } else {
            format!("{stem}-{sequence}.zsh-history.gz")
        };
        let output = directory.join(file_name);

        match temp.persist_noclobber(&output) {
            Ok(file) => {
                file.sync_all()
                    .with_context(|| format!("failed to sync {}", output.display()))?;
                return Ok(output);
            }
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
                temp = error.file;
            }
            Err(error) => {
                return Err(error.error)
                    .with_context(|| format!("failed to publish {}", output.display()));
            }
        }
    }

    bail!(
        "could not choose a unique archive name in {}",
        directory.display()
    )
}

fn parse_extended_history_timestamp(line: &[u8]) -> Option<i64> {
    let fields = line.strip_prefix(b": ")?;
    let timestamp_end = fields.iter().position(|byte| *byte == b':')?;
    let timestamp = &fields[..timestamp_end];
    if timestamp.is_empty()
        || !timestamp
            .iter()
            .enumerate()
            .all(|(index, byte)| byte.is_ascii_digit() || (index == 0 && *byte == b'-'))
    {
        return None;
    }

    let duration_and_command = &fields[timestamp_end + 1..];
    let duration_end = duration_and_command.iter().position(|byte| *byte == b';')?;
    let duration = &duration_and_command[..duration_end];
    if duration.is_empty() || !duration.iter().all(u8::is_ascii_digit) {
        return None;
    }

    std::str::from_utf8(timestamp).ok()?.parse().ok()
}
