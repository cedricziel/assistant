//! Device linking for the Signal interface.
//!
//! Without `--features signal` this function returns an informative error.
//! With `--features signal` it opens a SQLite store, initiates the presage
//! secondary-device linking flow, and prints a QR code for the user to scan
//! in the Signal app under **Settings → Linked Devices → Link a device**.

use std::path::Path;

use anyhow::Result;

/// Link this machine as a Signal secondary device.
///
/// Prints a QR code (and the raw URL) that the user must scan in the Signal
/// app.  The function awaits until the device linking handshake completes or
/// an error occurs.
///
/// # Errors
///
/// - Without `--features signal`: always returns a compilation-hint error.
/// - With `--features signal`: returns an error if the store cannot be opened
///   or the linking handshake fails.
pub async fn link_device(store_path: &Path, device_name: &str) -> Result<()> {
    #[cfg(not(feature = "signal"))]
    {
        let _ = (store_path, device_name);
        anyhow::bail!(
            "The Signal interface requires recompiling with `--features signal`.\n\
             Rebuild with:\n\
             \n\
             cargo build -p assistant-interface-signal --features signal\n\
             \n\
             See crates/interface-signal/Cargo.toml for the presage git dependencies."
        );
    }

    #[cfg(feature = "signal")]
    {
        use anyhow::Context as _;
        use futures::{channel::oneshot, future};
        use presage::libsignal_service::configuration::SignalServers;
        use presage::Manager;
        use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore};
        use std::str::FromStr as _;

        tracing::info!(
            store_path = %store_path.display(),
            device_name,
            "Opening signal store for device linking"
        );

        // Ensure the parent directory exists before SQLite tries to create the file.
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create signal store directory: {}",
                    parent.display()
                )
            })?;
        }

        // presage-store-sqlite uses a SQLite URL; create_if_missing must be
        // explicit — the default is false, causing SQLITE_CANTOPEN on a fresh
        // install where the file does not yet exist.
        let db_url = format!("sqlite://{}", store_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)
            .with_context(|| format!("Invalid signal store path: {}", store_path.display()))?
            .create_if_missing(true);
        let store = SqliteStore::open_with_options(options, OnNewIdentity::Trust)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to open signal store at {}: {e}",
                    store_path.display()
                )
            })?;

        let (provisioning_tx, provisioning_rx) = oneshot::channel();

        println!("Initiating Signal device linking…");
        println!("A QR code will appear below. Scan it in:");
        println!("  Signal app → Settings → Linked Devices → Link a device\n");

        let (link_result, qr_result) = future::join(
            Manager::link_secondary_device(
                store,
                SignalServers::Production,
                device_name.to_string(),
                provisioning_tx,
            ),
            async move {
                let url = provisioning_rx
                    .await
                    .map_err(|_| anyhow::anyhow!("Provisioning channel closed unexpectedly"))?;
                qr2term::print_qr(url.to_string())
                    .map_err(|e| anyhow::anyhow!("Failed to render QR code: {e}"))?;
                println!("\nOr paste this URL into Signal on your mobile device:\n{url}");
                println!("\nWaiting for the linking handshake to complete…");
                Ok::<_, anyhow::Error>(())
            },
        )
        .await;

        qr_result?;
        link_result.map_err(|e| anyhow::anyhow!("Device linking failed: {e}"))?;

        println!("\nDevice linked successfully!");
        println!("Run `assistant-signal run` to start the listener.");
        Ok(())
    }
}
