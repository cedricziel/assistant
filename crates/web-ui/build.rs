// Build scripts are a special context: failures here mean the package cannot
// be built, so `unwrap`/`expect`/`panic!` are the *correct* error signaling
// mechanism (cargo surfaces them to the operator with full context). The
// workspace-lint-policy deny doesn't apply to build scripts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Build script for `assistant-web-ui`.
//!
//! Never invokes Flutter. The web app is built by a separate, explicit step
//! (`make build-web`, or `flutter build web --release` in `app/`) that writes
//! `app/build/web/`; this script only prepares that output for embedding via
//! `rust-embed` (see `src/flutter_assets.rs`):
//!
//! - If `app/build/web/index.html` exists, it is embedded as-is after the
//!   service-worker version is injected.
//! - Otherwise a minimal placeholder `index.html` is written so the crate still
//!   compiles, but the embedded UI is not functional.

use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let app_dir = Path::new(&manifest_dir).join("..").join("..").join("app");
    let web_out = app_dir.join("build").join("web");

    // Re-run when the web build output or the pubspec version changes.
    println!("cargo:rerun-if-changed={}", web_out.display());
    println!(
        "cargo:rerun-if-changed={}",
        app_dir.join("pubspec.yaml").display()
    );
    println!("cargo:rerun-if-changed=build.rs");

    if web_out.join("index.html").exists() {
        if web_out.join("sw.js").exists() {
            inject_sw_version(&app_dir, &web_out);
        }
        return;
    }

    println!(
        "cargo:warning=app/build/web not found — embedding a placeholder page. \
         Run `make build-web` before building to embed the web UI."
    );
    std::fs::create_dir_all(&web_out).unwrap();
    std::fs::write(
        web_out.join("index.html"),
        b"<!DOCTYPE html><html><head><title>Assistant</title></head>\
          <body><p>Flutter web UI not built. \
          Run <code>make build-web</code> and rebuild.</p>\
          </body></html>",
    )
    .unwrap();
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
