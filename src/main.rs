mod archive;
mod config;
mod logging;
mod paths;

use anyhow::{Context, Result};
use archive::ArchiveReport;
use chrono::{Days, Local, NaiveDate};
use clap::{Args, Parser, Subcommand};
use paths::AppPaths;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "zsh-history-backup",
    version,
    about = "Back up and export Zsh EXTENDED_HISTORY without modifying the source"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Config file path (default: $XDG_CONFIG_HOME/zsh-history-backup/config.toml)"
    )]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Create a complete gzip snapshot of the configured history file")]
    Backup(BackupArgs),
    #[command(about = "Export entries from one local calendar date as gzip")]
    Export(ExportArgs),
}

#[derive(Args)]
struct BackupArgs {
    #[arg(long, value_parser = parse_name, help = "Optional safe label for the backup")]
    name: Option<String>,
}

#[derive(Args)]
struct ExportArgs {
    #[arg(
        long,
        conflicts_with = "date",
        required_unless_present = "date",
        help = "Export one date N days before today; 0 means today"
    )]
    days_ago: Option<u32>,
    #[arg(
        long,
        conflicts_with = "days_ago",
        required_unless_present = "days_ago",
        value_parser = parse_date,
        help = "Export one date in YYYY-MM-DD format"
    )]
    date: Option<NaiveDate>,
    #[arg(long, value_parser = parse_name, help = "Optional safe label for the export")]
    name: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zsh-history-backup: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover(cli.config.as_deref())?;
    paths.ensure_layout()?;

    let operation = cli.command.operation_name();
    let result = execute(cli.command, &paths);

    match result {
        Ok(report) => {
            let log_message = report.log_message(&paths.history_file);
            logging::append(&paths.log_file, "INFO", operation, &log_message).with_context(
                || {
                    format!(
                        "archive was created at {}, but the log could not be updated",
                        report.output_path().display()
                    )
                },
            )?;
            print_report(&report);
            Ok(())
        }
        Err(error) => {
            let log_message = format!(
                "source={} error={}",
                paths.history_file.display(),
                clean_log_value(&format!("{error:#}"))
            );
            if let Err(log_error) =
                logging::append(&paths.log_file, "ERROR", operation, &log_message)
            {
                eprintln!(
                    "warning: could not update {}: {log_error}",
                    paths.log_file.display()
                );
            }
            Err(error)
        }
    }
}

fn execute(command: Command, paths: &AppPaths) -> Result<ArchiveReport> {
    match command {
        Command::Backup(args) => archive::backup(paths, args.name.as_deref()),
        Command::Export(args) => {
            let target_date = match (args.days_ago, args.date) {
                (Some(days), None) => Local::now()
                    .date_naive()
                    .checked_sub_days(Days::new(u64::from(days)))
                    .context("days-ago is outside the supported date range")?,
                (None, Some(date)) => date,
                _ => unreachable!("clap enforces exactly one date selector"),
            };
            archive::export_date(paths, target_date, args.name.as_deref())
        }
    }
}

impl Command {
    fn operation_name(&self) -> &'static str {
        match self {
            Self::Backup(_) => "backup",
            Self::Export(_) => "export",
        }
    }
}

fn print_report(report: &ArchiveReport) {
    match report {
        ArchiveReport::Backup {
            output,
            source_bytes,
        } => {
            println!("backup created: {}", output.display());
            println!("source bytes: {source_bytes}");
        }
        ArchiveReport::Export {
            output,
            target_date,
            selected_entries,
            timestamped_entries,
            source_bytes,
        } => {
            println!("export created: {}", output.display());
            println!("date: {target_date}");
            println!("entries exported: {selected_entries}");
            println!("timestamped entries scanned: {timestamped_entries}");
            println!("source bytes scanned: {source_bytes}");
        }
    }
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "date must use YYYY-MM-DD format".to_owned())
}

fn parse_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("name cannot be empty".to_owned());
    }
    if trimmed.chars().count() > 80 {
        return Err("name cannot exceed 80 characters".to_owned());
    }
    if trimmed.starts_with('.') {
        return Err("name cannot start with a dot".to_owned());
    }

    let mut normalized = String::with_capacity(trimmed.len());
    let mut previous_was_dash = false;
    for character in trimmed.chars() {
        if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
            normalized.push(character);
            previous_was_dash = character == '-';
        } else if character.is_whitespace() {
            if !previous_was_dash {
                normalized.push('-');
                previous_was_dash = true;
            }
        } else {
            return Err(format!(
                "name contains unsupported character: {character:?}; use letters, numbers, spaces, dot, dash, or underscore"
            ));
        }
    }

    if normalized == "." || normalized == ".." {
        return Err("name must not be . or ..".to_owned());
    }

    Ok(normalized)
}

fn clean_log_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect()
}
