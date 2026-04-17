//! `assistant backup` and `assistant restore` subcommand implementations.

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use assistant_backup::{
    BackupEngine, BackupOptions, BackupResult, RestoreEngine, RestoreOptions, RestoreResult,
    list_backups,
    paths::{default_backups_dir, default_install_dir},
};

// ── Clap command definitions ──────────────────────────────────────────────────

/// Create and manage installation backups.
#[derive(clap::Args)]
pub struct BackupArgs {
    #[command(subcommand)]
    pub command: Option<BackupCommand>,

    /// Output path for the backup archive.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Override the installation root to back up.
    #[arg(long)]
    pub install_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum BackupCommand {
    /// List backup archives in the default backup directory.
    List {
        /// Directory to scan for archives.
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

/// Restore an installation from a backup archive.
#[derive(clap::Args)]
pub struct RestoreArgs {
    /// Path to the `.tar.gz` backup archive.
    pub archive: PathBuf,

    /// Skip interactive confirmation (for scripted use).
    #[arg(short, long)]
    pub force: bool,

    /// Override the restore target directory.
    #[arg(long)]
    pub install_dir: Option<PathBuf>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Handle `assistant backup [--output <path>] [--install-dir <path>]`
pub async fn cmd_backup(args: &BackupArgs) -> Result<()> {
    // If a subcommand was provided, delegate to it
    if let Some(BackupCommand::List { dir }) = &args.command {
        return cmd_backup_list(dir.as_deref()).await;
    }

    let install_dir = args.install_dir.clone().unwrap_or_else(default_install_dir);

    let output_path = match &args.output {
        Some(p) => p.clone(),
        None => {
            use assistant_backup::paths::{default_archive_name, default_backups_dir};
            default_backups_dir().join(default_archive_name())
        }
    };

    let opts = BackupOptions {
        install_dir,
        output_path,
        db_path: None,
    };

    let result = BackupEngine::new().run(opts).await?;
    print_backup_result(&result);
    Ok(())
}

fn print_backup_result(result: &BackupResult) {
    let size_mb = result.archive_size as f64 / 1_048_576.0;
    println!("Backup created: {}", result.output_path.display());
    println!("  Size:    {:.1} MB", size_mb);
    println!("  Files:   {}", result.entry_count);
    println!("  Created: {}", result.manifest.created_at);
}

/// Handle `assistant backup list [--dir <path>]`
async fn cmd_backup_list(dir: Option<&std::path::Path>) -> Result<()> {
    let backup_dir = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(default_backups_dir);

    let infos = list_backups(&backup_dir).await?;

    if infos.is_empty() {
        println!("No backups found in {}", backup_dir.display());
        println!("Run 'assistant backup' to create one.");
        return Ok(());
    }

    println!("Backups in {}\n", backup_dir.display());
    for info in &infos {
        let size_mb = info.archive_size as f64 / 1_048_576.0;
        println!(
            "  {}   {:.1} MB   {}   {} files",
            info.path.file_name().unwrap_or_default().to_string_lossy(),
            size_mb,
            info.created_at,
            info.entry_count,
        );
    }
    println!("\n{} backup(s) found.", infos.len());
    Ok(())
}

/// Handle `assistant restore <archive> [--force] [--install-dir <path>]`
pub async fn cmd_restore(args: &RestoreArgs) -> Result<()> {
    let install_dir = args.install_dir.clone().unwrap_or_else(default_install_dir);

    let opts = RestoreOptions {
        archive_path: args.archive.clone(),
        install_dir,
        force: args.force,
    };

    let result = RestoreEngine::new().run(opts).await?;
    print_restore_result(&result);
    Ok(())
}

fn print_restore_result(result: &RestoreResult) {
    println!(
        "Restore complete: {} files restored.",
        result.restored_count
    );
    for w in &result.warnings {
        eprintln!("warning: {}", w);
    }
}
