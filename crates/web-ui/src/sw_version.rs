//! Service-worker version injection helper.
//!
//! Substitutes the `__APP_VERSION__` placeholder in `app/build/web/sw.js`
//! with the package version. Used by `build.rs`; exposed publicly so the
//! integration test in `tests/sw_version_injection.rs` can pin the contract.
//!
//! The function is intentionally idempotent: re-runs on an already-injected
//! file are a silent no-op (Cargo runs `build.rs` once per compile target,
//! so the same file may be processed multiple times in a single build). The
//! only fatal case is a missing placeholder *and* no recognizable existing
//! version comment — that means someone removed the load-bearing marker.

use std::fmt;

const PLACEHOLDER: &str = "__APP_VERSION__";

/// Error returned by [`inject_sw_version`] when the source has neither the
/// placeholder nor an already-injected version line.
#[derive(Debug)]
pub struct MissingPlaceholder;

impl fmt::Display for MissingPlaceholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "sw.js is missing the `__APP_VERSION__` placeholder — see the \
             top-of-file comment in app/web/sw.js documenting the contract.",
        )
    }
}

impl std::error::Error for MissingPlaceholder {}

/// Replaces `__APP_VERSION__` in `src` with `version`.
///
/// - If `src` already contains a `// vX.Y.Z` comment (any version), returns
///   `src` unchanged. Idempotent for repeat builds.
/// - Otherwise, requires the placeholder; returns [`MissingPlaceholder`]
///   if neither is present.
pub fn inject_sw_version(src: &str, version: &str) -> Result<String, MissingPlaceholder> {
    if src.contains(PLACEHOLDER) {
        return Ok(src.replace(PLACEHOLDER, version));
    }
    // Already-injected file from a previous build run. Recognise the comment
    // pattern `// v<digits>.<digits>...` so we don't flag idempotent re-runs.
    if has_existing_version_comment(src) {
        return Ok(src.to_owned());
    }
    Err(MissingPlaceholder)
}

fn has_existing_version_comment(src: &str) -> bool {
    src.lines().any(|line| {
        let trimmed = line.trim_start();
        // Match e.g. `// v1.2.3`, `// v0.1.146 — ...` etc.
        let after_slash = trimmed.strip_prefix("//").map(str::trim_start);
        if let Some(rest) = after_slash
            && let Some(after_v) = rest.strip_prefix('v')
        {
            return after_v
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_placeholder() {
        let out = inject_sw_version("// v__APP_VERSION__\n", "0.1.0").unwrap();
        assert_eq!(out, "// v0.1.0\n");
    }

    #[test]
    fn idempotent_on_already_injected() {
        let src = "// v0.1.0 — replaced by build.rs.\n";
        assert_eq!(inject_sw_version(src, "0.2.0").unwrap(), src);
    }

    #[test]
    fn errors_when_no_marker_at_all() {
        let result = inject_sw_version("// just a service worker\n", "0.1.0");
        assert!(matches!(result, Err(MissingPlaceholder)));
    }
}
