//! Unified-diff rendering for `/review` skill refinement display (#7).
//!
//! The orchestrator emits skill refinement proposals (`SKILL.md` replacements)
//! that the user reviews via the `/review` CLI command. Until #7 was fixed,
//! the command only showed the proposal's id / target / rationale — to see
//! what would actually change the user had to manually `diff` the file
//! against the proposal. Now we render a unified diff inline with ANSI
//! colour so the change is visible at a glance.

use std::fmt::Write;

use similar::{ChangeTag, TextDiff};

/// ANSI escape codes. Kept tiny so we don't pull in a colour crate just for
/// three sequences.
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_RESET: &str = "\x1b[0m";

/// Render a unified diff between `current` and `proposed` SKILL.md content as
/// a coloured, terminal-printable string. The output is line-oriented:
///
/// - `+ ...` (green) for additions.
/// - `- ...` (red) for deletions.
/// - `  ...` (dim) for surrounding context, capped at three lines on either
///   side of each hunk.
///
/// `colour` controls whether ANSI escapes are emitted. Tests pass `false` to
/// keep snapshot output stable.
pub fn render_unified_diff(current: &str, proposed: &str, colour: bool) -> String {
    let diff = TextDiff::from_lines(current, proposed);

    let mut out = String::new();
    for group in diff.grouped_ops(3) {
        for op in group {
            for change in diff.iter_changes(&op) {
                let (sign, ansi) = match change.tag() {
                    ChangeTag::Insert => ("+", ANSI_GREEN),
                    ChangeTag::Delete => ("-", ANSI_RED),
                    ChangeTag::Equal => (" ", ANSI_DIM),
                };
                let line = change.value();
                // `change.value()` already includes the trailing newline when
                // the source did; preserve it verbatim.
                if colour {
                    let _ = write!(out, "{ansi}{sign} {line}{ANSI_RESET}");
                } else {
                    let _ = write!(out, "{sign} {line}");
                }
                if !line.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }

    if out.is_empty() {
        out.push_str("(no changes)\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_unified_diff_marks_inserts_and_deletes() {
        let current = "alpha\nbeta\ngamma\n";
        let proposed = "alpha\nBETA\ngamma\n";
        let diff = render_unified_diff(current, proposed, false);

        assert!(
            diff.contains("- beta"),
            "expected '- beta' marker, got:\n{diff}"
        );
        assert!(
            diff.contains("+ BETA"),
            "expected '+ BETA' marker, got:\n{diff}"
        );
        // Surrounding context should appear.
        assert!(
            diff.contains("  alpha"),
            "expected surrounding context, got:\n{diff}"
        );
        assert!(
            diff.contains("  gamma"),
            "expected surrounding context, got:\n{diff}"
        );
    }

    #[test]
    fn render_unified_diff_no_changes_returns_sentinel() {
        let same = "alpha\nbeta\n";
        let diff = render_unified_diff(same, same, false);
        assert_eq!(diff.trim(), "(no changes)");
    }

    #[test]
    fn render_unified_diff_pure_addition() {
        let current = "";
        let proposed = "new line\n";
        let diff = render_unified_diff(current, proposed, false);
        assert!(
            diff.contains("+ new line"),
            "expected '+ new line', got:\n{diff}"
        );
    }

    #[test]
    fn render_unified_diff_pure_deletion() {
        let current = "old line\n";
        let proposed = "";
        let diff = render_unified_diff(current, proposed, false);
        assert!(
            diff.contains("- old line"),
            "expected '- old line', got:\n{diff}"
        );
    }

    #[test]
    fn render_unified_diff_emits_ansi_when_requested() {
        let current = "a\n";
        let proposed = "b\n";
        let diff = render_unified_diff(current, proposed, true);
        assert!(
            diff.contains(ANSI_GREEN),
            "expected ANSI green escape, got:\n{diff:?}"
        );
        assert!(
            diff.contains(ANSI_RED),
            "expected ANSI red escape, got:\n{diff:?}"
        );
        assert!(
            diff.contains(ANSI_RESET),
            "expected ANSI reset escape, got:\n{diff:?}"
        );
    }

    #[test]
    fn render_unified_diff_omits_ansi_when_disabled() {
        let current = "a\n";
        let proposed = "b\n";
        let diff = render_unified_diff(current, proposed, false);
        assert!(
            !diff.contains('\x1b'),
            "expected no ANSI escapes, got:\n{diff:?}"
        );
    }
}
