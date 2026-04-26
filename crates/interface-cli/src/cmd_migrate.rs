//! `assistant migrate finalize` — recover installs that were partially
//! migrated to the multi-org layout but never cut over.
//!
//! See `openspec/changes/multi-org-runtime-cutover/specs/runtime-space-storage/spec.md`
//! requirement "`assistant migrate finalize` recovers stuck installs" for the
//! contract. The high-level flow:
//!
//! 1. Detect any running orchestrator/web-ui processes; refuse unless `--force`.
//! 2. WAL-checkpoint the live `assistant.db`.
//! 3. Overwrite `space.db` with the checkpointed `assistant.db` content.
//! 4. Rename `assistant.db` → `assistant.db.legacy`, drop `*-shm` / `*-wal`.
//! 5. Print restart instructions.
//!
//! No-op when `assistant.db` is already absent (only `assistant.db.legacy` and
//! `space.db` remain).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sqlx::sqlite::SqlitePoolOptions;
use tracing::{info, warn};

use assistant_storage::OrgPoolFactory;

/// Information about an `assistant` process that prevents `finalize` from
/// running safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunningAssistant {
    pub pid: u32,
    pub cmd: String,
}

/// Pure helper used by tests: scan a list of `(pid, command-line)` pairs and
/// return any that look like a live orchestrator or web-ui owned by the
/// current user.
///
/// Match rule: any element whose joined command-line contains
/// `assistant orchestrator run` **or** `assistant webui` (with or without the
/// `serve` subcommand). Substring match is intentional — it catches both
/// `/usr/local/bin/assistant orchestrator run --no-repl` and
/// `op run -- assistant webui serve`.
pub(crate) fn find_running_assistant_processes(
    cmdlines: &[(u32, String)],
) -> Vec<RunningAssistant> {
    cmdlines
        .iter()
        .filter(|(_, cmd)| {
            cmd.contains("assistant orchestrator run") || cmd.contains("assistant webui")
        })
        .map(|(pid, cmd)| RunningAssistant {
            pid: *pid,
            cmd: cmd.clone(),
        })
        .collect()
}

/// Collect every running process and its command-line as a `(pid, cmd)` pair.
///
/// Uses `sysinfo` so the lookup is cross-platform (Linux/macOS/Windows). The
/// CLI itself is excluded by filtering on its own pid.
fn collect_process_cmdlines() -> Vec<(u32, String)> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::Always),
    );

    let self_pid = std::process::id();

    sys.processes()
        .iter()
        .filter_map(|(pid, proc)| {
            let pid_u32 = pid.as_u32();
            if pid_u32 == self_pid {
                return None;
            }
            let cmd: Vec<String> = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            if cmd.is_empty() {
                None
            } else {
                Some((pid_u32, cmd.join(" ")))
            }
        })
        .collect()
}

/// Run `assistant migrate finalize` against `base_path` (default
/// `~/.assistant`). When `force` is true, the running-process check is
/// skipped — operators must guarantee no live writers themselves.
pub async fn cmd_migrate_finalize(force: bool, base_path: &Path) -> Result<()> {
    cmd_migrate_finalize_with_detector(force, base_path, &collect_process_cmdlines).await
}

/// Same as [`cmd_migrate_finalize`] but the running-process detector is
/// injected so tests can simulate live orchestrator/webui processes without
/// actually spawning them.
pub(crate) async fn cmd_migrate_finalize_with_detector(
    force: bool,
    base_path: &Path,
    detect: &dyn Fn() -> Vec<(u32, String)>,
) -> Result<()> {
    let legacy_db = base_path.join("assistant.db");

    // No-op path: nothing to finalize.
    if !legacy_db.exists() {
        println!("already finalized — nothing to do");
        return Ok(());
    }

    if !force {
        let running = find_running_assistant_processes(&detect());
        if !running.is_empty() {
            for proc in &running {
                eprintln!("refusing to finalize: pid {} is {}", proc.pid, proc.cmd);
            }
            bail!(
                "{} assistant process(es) still running — stop them first or pass --force",
                running.len()
            );
        }
    }

    // WAL checkpoint via sqlx so the live DB content is fully captured in the
    // main file before we copy it. `wal_checkpoint(TRUNCATE)` blocks until all
    // outstanding pages are written and the WAL is truncated.
    let conn_url = format!("sqlite://{}", legacy_db.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&conn_url)
        .await
        .with_context(|| format!("opening {} for checkpoint", legacy_db.display()))?;
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
        .execute(&pool)
        .await
        .context("checkpointing WAL on legacy assistant.db")?;
    pool.close().await;
    info!("WAL-checkpointed {}", legacy_db.display());

    // Copy assistant.db → space.db (overwriting the stale snapshot).
    let space_db = OrgPoolFactory::new(base_path).space_db_path("default", "default");
    if let Some(parent) = space_db.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating {} parent directory", space_db.display()))?;
    }
    tokio::fs::copy(&legacy_db, &space_db)
        .await
        .with_context(|| format!("copying {} → {}", legacy_db.display(), space_db.display()))?;
    info!("copied {} → {}", legacy_db.display(), space_db.display());

    // Atomic cutover: rename + sidecar removal, mirroring `migrate_database`.
    let renamed: PathBuf = base_path.join("assistant.db.legacy");
    tokio::fs::rename(&legacy_db, &renamed)
        .await
        .with_context(|| format!("renaming {} → {}", legacy_db.display(), renamed.display()))?;
    info!("renamed {} → {}", legacy_db.display(), renamed.display());

    for suffix in &["-wal", "-shm"] {
        let sidecar = base_path.join(format!("assistant.db{suffix}"));
        if sidecar.exists()
            && let Err(e) = tokio::fs::remove_file(&sidecar).await
        {
            warn!(
                path = %sidecar.display(),
                error = %e,
                "failed to remove legacy sidecar"
            );
        }
    }

    println!();
    println!("Finalize complete:");
    println!("  - {} updated from live legacy DB", space_db.display());
    println!("  - assistant.db renamed to {}", renamed.display());
    println!();
    println!(
        "Restart services now (e.g. `systemctl --user start assistant-{{slack,matrix,web-ui}}`)."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -- find_running_assistant_processes -----------------------------------

    #[test]
    fn detects_orchestrator_in_cmdline() {
        let cmds = vec![
            (
                1234,
                "/usr/local/bin/assistant orchestrator run --no-repl".to_string(),
            ),
            (5678, "/bin/bash -c something-else".to_string()),
        ];
        let found = find_running_assistant_processes(&cmds);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 1234);
    }

    #[test]
    fn detects_webui_in_cmdline() {
        let cmds = vec![(
            7777,
            "op run -- /usr/local/bin/assistant webui serve --listen 0.0.0.0:8080".to_string(),
        )];
        let found = find_running_assistant_processes(&cmds);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 7777);
    }

    #[test]
    fn ignores_unrelated_processes() {
        let cmds = vec![
            (1, "/sbin/init".to_string()),
            (42, "vim /etc/hosts".to_string()),
            (99, "assistant migrate finalize".to_string()),
        ];
        let found = find_running_assistant_processes(&cmds);
        assert!(found.is_empty(), "unrelated processes should not match");
    }

    #[test]
    fn detects_multiple_running_services() {
        let cmds = vec![
            (
                1,
                "assistant orchestrator run --interfaces matrix".to_string(),
            ),
            (
                2,
                "assistant orchestrator run --interfaces slack".to_string(),
            ),
            (3, "assistant webui serve".to_string()),
        ];
        let found = find_running_assistant_processes(&cmds);
        assert_eq!(found.len(), 3, "should detect all three services");
    }

    // -- cmd_migrate_finalize ------------------------------------------------

    #[tokio::test]
    async fn finalize_is_noop_when_legacy_db_absent() {
        let dir = TempDir::new().unwrap();
        // No assistant.db exists.
        cmd_migrate_finalize(false, dir.path())
            .await
            .expect("finalize should succeed with no-op when assistant.db is absent");

        // Nothing should have been created.
        assert!(!dir.path().join("assistant.db").exists());
        assert!(!dir.path().join("assistant.db.legacy").exists());
    }

    #[tokio::test]
    async fn finalize_succeeds_on_idle_host() {
        let dir = TempDir::new().unwrap();
        let legacy_db = dir.path().join("assistant.db");

        // Seed a real legacy DB so the WAL checkpoint has something to do.
        let storage = assistant_storage::StorageLayer::new(&legacy_db)
            .await
            .unwrap();
        drop(storage);

        // Pre-create a stale space.db (smaller than legacy) — finalize should
        // overwrite it.
        let space_db = OrgPoolFactory::new(dir.path()).space_db_path("default", "default");
        std::fs::create_dir_all(space_db.parent().unwrap()).unwrap();
        std::fs::write(&space_db, b"stale").unwrap();

        cmd_migrate_finalize(true, dir.path())
            .await
            .expect("finalize should succeed on idle host with --force");

        assert!(
            !legacy_db.exists(),
            "assistant.db should be renamed away after finalize"
        );
        assert!(
            dir.path().join("assistant.db.legacy").exists(),
            "assistant.db.legacy should exist after finalize"
        );
        let space_meta = std::fs::metadata(&space_db).unwrap();
        assert!(
            space_meta.len() > b"stale".len() as u64,
            "space.db should have been overwritten with the legacy DB content"
        );
    }

    /// Helper: seed a tempdir with a real legacy `assistant.db` containing the
    /// schema. Returns the legacy db path.
    async fn seed_legacy_install(dir: &TempDir) -> PathBuf {
        let legacy_db = dir.path().join("assistant.db");
        let storage = assistant_storage::StorageLayer::new(&legacy_db)
            .await
            .unwrap();
        drop(storage);
        legacy_db
    }

    /// Detector that always reports an orchestrator running — used to simulate
    /// "services up" without spawning a real process.
    fn fake_orchestrator_running() -> Vec<(u32, String)> {
        vec![(
            4242,
            "/usr/local/bin/assistant orchestrator run".to_string(),
        )]
    }

    /// Detector that always reports no assistant processes running.
    fn no_running_processes() -> Vec<(u32, String)> {
        vec![(1, "/sbin/init".to_string()), (42, "vim".to_string())]
    }

    /// Integration test: services-running refusal — finalize must bail when
    /// the detector reports a live orchestrator and `--force` is not set.
    #[tokio::test]
    async fn finalize_refuses_when_orchestrator_is_running() {
        let dir = TempDir::new().unwrap();
        let legacy_db = seed_legacy_install(&dir).await;

        let result =
            cmd_migrate_finalize_with_detector(false, dir.path(), &fake_orchestrator_running).await;

        let err = result.expect_err("finalize must refuse while orchestrator is running");
        let msg = format!("{err}");
        assert!(
            msg.contains("still running"),
            "error should explain the refusal, got: {msg}"
        );

        // The legacy DB must remain untouched on refusal.
        assert!(
            legacy_db.exists(),
            "assistant.db must not be moved when finalize refuses"
        );
        assert!(
            !dir.path().join("assistant.db.legacy").exists(),
            "assistant.db.legacy must not be created on refusal"
        );
    }

    /// Integration test: `--force` overrides the running-process check —
    /// finalize succeeds even when the detector reports live processes.
    #[tokio::test]
    async fn finalize_force_overrides_running_check() {
        let dir = TempDir::new().unwrap();
        let legacy_db = seed_legacy_install(&dir).await;

        cmd_migrate_finalize_with_detector(true, dir.path(), &fake_orchestrator_running)
            .await
            .expect("finalize must succeed with --force even when processes appear running");

        assert!(
            !legacy_db.exists(),
            "assistant.db should be renamed away after forced finalize"
        );
        assert!(
            dir.path().join("assistant.db.legacy").exists(),
            "assistant.db.legacy should exist after forced finalize"
        );
    }

    /// Integration test: success path with no running processes — finalize
    /// completes without `--force` when the detector reports a clean host.
    #[tokio::test]
    async fn finalize_success_path_with_idle_detector() {
        let dir = TempDir::new().unwrap();
        let legacy_db = seed_legacy_install(&dir).await;

        cmd_migrate_finalize_with_detector(false, dir.path(), &no_running_processes)
            .await
            .expect("finalize must succeed when detector reports an idle host");

        assert!(
            !legacy_db.exists(),
            "assistant.db should be renamed away after finalize"
        );
        let space_db = OrgPoolFactory::new(dir.path()).space_db_path("default", "default");
        assert!(
            space_db.exists(),
            "space.db must exist after a successful finalize"
        );
    }

    /// Integration test: no-op path — when `assistant.db` is absent the
    /// detector is never consulted and finalize prints its no-op message.
    #[tokio::test]
    async fn finalize_noop_path_does_not_invoke_detector() {
        let dir = TempDir::new().unwrap();

        // Counter that proves the detector was NOT called.
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let detector = || {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Vec::new()
        };

        cmd_migrate_finalize_with_detector(false, dir.path(), &detector)
            .await
            .expect("no-op finalize should always succeed");

        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "detector must not be invoked on the no-op path"
        );
        assert!(!dir.path().join("assistant.db").exists());
        assert!(!dir.path().join("assistant.db.legacy").exists());
    }

    /// Integration test: sidecar files (`*-shm`/`*-wal`) are removed when
    /// finalize cuts over, mirroring the post-conditions from the
    /// `migrate_database` rename step.
    #[tokio::test]
    async fn finalize_removes_legacy_sidecars() {
        let dir = TempDir::new().unwrap();
        let _ = seed_legacy_install(&dir).await;

        // Touch fake sidecars — sqlx may not have created them yet, but
        // operators occasionally hit a state where they linger.
        let shm = dir.path().join("assistant.db-shm");
        let wal = dir.path().join("assistant.db-wal");
        std::fs::write(&shm, b"shm-stub").unwrap();
        std::fs::write(&wal, b"wal-stub").unwrap();

        cmd_migrate_finalize_with_detector(true, dir.path(), &no_running_processes)
            .await
            .expect("finalize should succeed");

        assert!(!shm.exists(), "assistant.db-shm should be removed");
        assert!(!wal.exists(), "assistant.db-wal should be removed");
    }
}
