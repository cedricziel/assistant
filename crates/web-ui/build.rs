//! Build script for `assistant-web-ui`.
//!
//! Runs `flutter build web --release` inside the `app/` directory at the repo
//! root whenever Flutter sources change, producing `app/build/web/`.  The
//! output is then embedded into the binary at compile time via `rust-embed`
//! (see `src/flutter_assets.rs`).
//!
//! If the Flutter SDK is not installed:
//! - If a prior build already exists in `app/build/web/`, it is reused as-is.
//!   This is the cross-compilation path: the host pre-builds Flutter web before
//!   invoking `cross`, and the Docker container reuses the output without
//!   needing Flutter installed.
//! - Otherwise a minimal placeholder `index.html` is written so the crate
//!   still compiles, but the embedded UI will not be functional.

use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let app_dir = Path::new(&manifest_dir).join("..").join("..").join("app");
    let web_out = app_dir.join("build").join("web");

    // Tell Cargo to re-run this script when Flutter sources change.
    println!("cargo:rerun-if-changed={}", app_dir.join("lib").display());
    println!(
        "cargo:rerun-if-changed={}",
        app_dir.join("pubspec.yaml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        app_dir.join("pubspec.lock").display()
    );
    println!("cargo:rerun-if-changed={}", app_dir.join("web").display());
    println!("cargo:rerun-if-changed=build.rs");

    // Check whether the Flutter SDK is available.
    let flutter_available = Command::new("flutter")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !flutter_available {
        if web_out.join("index.html").exists() {
            // A prior `flutter build web` output is present (e.g. pre-built on
            // the host before invoking `cross`).  Reuse it rather than
            // overwriting with the stub.
            println!(
                "cargo:warning=Flutter SDK not found — reusing existing build at {}",
                web_out.display()
            );
        } else {
            println!(
                "cargo:warning=Flutter SDK not found — skipping `flutter build web`. \
                 Install Flutter (stable channel) and rebuild to embed the web UI."
            );
            // Write a placeholder so rust-embed has something to embed.
            std::fs::create_dir_all(&web_out).unwrap_or_default();
            std::fs::write(
                web_out.join("index.html"),
                b"<!DOCTYPE html><html><head><title>Assistant</title></head>\
                  <body><p>Flutter web UI not built. \
                  Install the Flutter SDK and run <code>cargo build</code> again.</p>\
                  </body></html>",
            )
            .unwrap_or_default();
        }
        return;
    }

    let status = Command::new("flutter")
        .args(["build", "web", "--release"])
        .current_dir(&app_dir)
        .status()
        .expect("failed to spawn `flutter build web`");

    if !status.success() {
        panic!(
            "`flutter build web --release` failed. \
             Check the Flutter installation and the sources under app/."
        );
    }
}
