// Build scripts are a special context: failures here mean the package cannot
// be built, so `unwrap`/`expect`/`panic!` are the *correct* error signaling
// mechanism (cargo surfaces them to the operator with full context). The
// workspace-lint-policy deny doesn't apply to build scripts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
            // Still inject the version so sw.js forces a browser update on
            // every release even when Flutter isn't available.
            if web_out.join("sw.js").exists() {
                inject_sw_version(&app_dir, &web_out);
            }
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

    // Skip the Flutter build if the output is already up-to-date.  Cargo
    // runs build.rs once per compilation target (e.g. aarch64-apple-darwin
    // and x86_64-apple-darwin both get separate executions).  The first
    // target does the full build; subsequent targets reuse the output.
    if web_out.join("index.html").exists() && !any_source_newer_than(&app_dir, &web_out) {
        println!(
            "cargo:warning=Flutter web output already up-to-date — skipping rebuild for this target"
        );
        if web_out.join("sw.js").exists() {
            inject_sw_version(&app_dir, &web_out);
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

    inject_sw_version(&app_dir, &web_out);
}

/// Returns `true` if any Flutter source file is newer than the built output,
/// meaning a rebuild is needed.  Returns `true` on any I/O error so we always
/// rebuild when in doubt.
fn any_source_newer_than(app_dir: &Path, web_out: &Path) -> bool {
    let out_time = match web_out
        .join("index.html")
        .metadata()
        .and_then(|m| m.modified())
    {
        Ok(t) => t,
        Err(_) => return true,
    };

    for path in &[app_dir.join("pubspec.yaml"), app_dir.join("pubspec.lock")] {
        if path
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t > out_time)
            .unwrap_or(true)
        {
            return true;
        }
    }

    for dir in &[app_dir.join("lib"), app_dir.join("web")] {
        if dir_has_newer(dir, out_time) {
            return true;
        }
    }

    false
}

fn dir_has_newer(dir: &Path, than: std::time::SystemTime) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if dir_has_newer(&path, than) {
                return true;
            }
        } else if path
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t > than)
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Read the `version:` field from `pubspec.yaml` and substitute the
/// `__APP_VERSION__` placeholder in the built `sw.js`.
///
/// Causes the browser to detect a new service worker script on every
/// version bump rather than waiting for its 24-hour byte-diff check.
///
/// Idempotent: re-runs on an already-injected file are silent. The build
/// FAILS if `sw.js` exists but contains neither the placeholder nor an
/// existing version comment — that would mean someone removed the
/// load-bearing marker and the SW would ship un-versioned.
fn inject_sw_version(app_dir: &Path, web_out: &Path) {
    let pubspec = match std::fs::read_to_string(app_dir.join("pubspec.yaml")) {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not read pubspec.yaml for sw.js version injection: {e}");
        }
    };

    // Parse `version: 1.2.3` or `version: 1.2.3+4` (strip build metadata).
    let Some(version) = pubspec.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("version:").map(|v| {
            let v = v.trim();
            v.split('+').next().unwrap_or(v).trim().to_owned()
        })
    }) else {
        panic!(
            "Could not parse `version:` from pubspec.yaml — sw.js version injection cannot proceed"
        );
    };

    let sw_path = web_out.join("sw.js");
    let sw_src = match std::fs::read_to_string(&sw_path) {
        Ok(s) => s,
        Err(e) => {
            panic!("Could not read built sw.js for version injection: {e}");
        }
    };

    match sw_version::inject_sw_version(&sw_src, &version) {
        Ok(injected) if injected == sw_src => {
            // Idempotent re-run; nothing to do.
        }
        Ok(injected) => {
            if let Err(e) = std::fs::write(&sw_path, injected) {
                panic!("Could not write sw.js after version injection: {e}");
            }
            println!("cargo:warning=sw.js version injected: {version}");
        }
        Err(e) => {
            panic!("{e}");
        }
    }
}

#[path = "src/sw_version.rs"]
mod sw_version;
