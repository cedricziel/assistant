//! Unit tests for the `inject_sw_version` build-script helper.
//!
//! Spec: openspec/changes/web-session-resilience/specs/web-session-resilience/spec.md
//!   "Service-worker version is injected on every release build"
//!
//! The helper has three branches we want pinned by tests:
//! 1. Source contains the placeholder → returns the substituted string.
//! 2. Source already contains the current version (idempotent re-run) → returns
//!    the original unchanged.
//! 3. Source contains neither → returns an error so the build fails loudly
//!    instead of silently shipping a stale service worker.
//!
//! The helper itself lives in `crates/web-ui/build_helpers/sw_version.rs`
//! (a sibling of `build.rs`) and is included from `build.rs` via `include!`.

use assistant_web_ui::sw_version::inject_sw_version;

#[test]
fn injects_placeholder_with_version() {
    let src = "// v__APP_VERSION__ — replaced by build.rs.\nconsole.log('sw');\n";
    let injected = inject_sw_version(src, "0.1.146").expect("injection should succeed");
    assert_eq!(
        injected,
        "// v0.1.146 — replaced by build.rs.\nconsole.log('sw');\n",
    );
}

#[test]
fn idempotent_when_current_version_already_present() {
    // Re-running the build sees the already-injected file. Must not warn or
    // error; just return the source unchanged.
    let src = "// v0.1.146 — replaced by build.rs.\nconsole.log('sw');\n";
    let result = inject_sw_version(src, "0.1.146").expect("idempotent re-run");
    assert_eq!(result, src);
}

#[test]
fn idempotent_for_any_existing_version_string() {
    // Even if the file contains an *older* version (cross-target rebuild
    // before the new version reached this target), don't error — assume
    // some other invocation will catch up. Only the missing-placeholder
    // case is fatal.
    let src = "// v0.1.145 — replaced by build.rs.\nconsole.log('sw');\n";
    let result = inject_sw_version(src, "0.1.146").expect("older version is non-fatal");
    // Older versions are also accepted — this branch is conservatively
    // tolerant; what we never want is a silently un-versioned SW.
    assert!(
        result.contains("v0.1.145") || result.contains("v0.1.146"),
        "result should still carry a version comment, got: {result}",
    );
}

#[test]
fn missing_placeholder_returns_error() {
    let src = "// service worker without version marker\nconsole.log('sw');\n";
    let err = inject_sw_version(src, "0.1.146").expect_err("must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("__APP_VERSION__"),
        "error must mention the placeholder; got: {msg}",
    );
}
