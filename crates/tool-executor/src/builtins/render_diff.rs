//! `code_diff` render data for `file-edit` and `file-write` (design D6).
//!
//! The diff is computed on the server from the old and new file content, so
//! the client needs no diff library. Output is capped by the `code_diff`
//! render budget (design D4): at most [`MAX_DIFF_LINES`] lines of at most
//! [`MAX_LINE_CHARS`] characters.

use std::path::Path;

use assistant_core::tool_render::{
    CodeDiffData, DiffHunk, DiffLine, DiffOp, DiffOperation, ToolRenderData,
};
use similar::{ChangeTag, TextDiff};

use super::render_lang::language_for_path;

/// Most diff lines (context included) published across all hunks.
const MAX_DIFF_LINES: usize = 400;
/// Most characters of one diff line, the `…` cut marker included.
const MAX_LINE_CHARS: usize = 512;
/// Unchanged lines kept around each change.
const CONTEXT_RADIUS: usize = 3;

/// The result of [`unified_hunks`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hunks {
    pub hunks: Vec<DiffHunk>,
    /// Added lines in the whole diff, including any cut from `hunks`.
    pub added: u64,
    /// Removed lines in the whole diff, including any cut from `hunks`.
    pub removed: u64,
    /// `true` when lines or characters were cut from `hunks`.
    pub truncated: bool,
}

/// Unified-diff hunks from `old` to `new` with three lines of context.
///
/// At most `max_lines` lines are kept across all hunks and each line is cut
/// to `max_line_chars` characters including a `…` marker, always on a
/// character boundary.
/// `added` and `removed` count the whole diff, not only the kept lines.
pub(crate) fn unified_hunks(
    old: &str,
    new: &str,
    max_lines: usize,
    max_line_chars: usize,
) -> Hunks {
    let diff = TextDiff::from_lines(old, new);
    let mut out = Hunks {
        hunks: Vec::new(),
        added: 0,
        removed: 0,
        truncated: false,
    };
    let mut remaining = max_lines;

    for group in diff.grouped_ops(CONTEXT_RADIUS) {
        let (mut old_start, mut old_end, mut new_start, mut new_end) =
            (usize::MAX, 0, usize::MAX, 0);
        let mut lines = Vec::new();
        for op in &group {
            let (_, old_range, new_range) = op.as_tag_tuple();
            old_start = old_start.min(old_range.start);
            old_end = old_end.max(old_range.end);
            new_start = new_start.min(new_range.start);
            new_end = new_end.max(new_range.end);
            for change in diff.iter_changes(op) {
                let line_op = match change.tag() {
                    ChangeTag::Equal => DiffOp::Context,
                    ChangeTag::Insert => {
                        out.added += 1;
                        DiffOp::Add
                    }
                    ChangeTag::Delete => {
                        out.removed += 1;
                        DiffOp::Remove
                    }
                };
                if remaining == 0 {
                    out.truncated = true;
                    continue;
                }
                remaining -= 1;
                let (text, cut) = cut_line(change.value(), max_line_chars);
                out.truncated |= cut;
                lines.push(DiffLine { op: line_op, text });
            }
        }
        if !lines.is_empty() {
            out.hunks.push(DiffHunk {
                // Unified diffs number lines from 1 (0 for an empty side).
                old_start: one_based(old_start, old_end),
                old_lines: (old_end - old_start.min(old_end)) as u64,
                new_start: one_based(new_start, new_end),
                new_lines: (new_end - new_start.min(new_end)) as u64,
                lines,
            });
        }
    }
    out
}

/// The `code_diff` render data for a change to `path`.
pub(crate) fn code_diff(
    path: &Path,
    operation: DiffOperation,
    old: &str,
    new: &str,
) -> ToolRenderData {
    let hunks = unified_hunks(old, new, MAX_DIFF_LINES, MAX_LINE_CHARS);
    ToolRenderData::CodeDiff(CodeDiffData {
        path: path.display().to_string(),
        language: language_for_path(path).map(str::to_string),
        operation,
        hunks: hunks.hunks,
        added: hunks.added,
        removed: hunks.removed,
        truncated: hunks.truncated,
    })
}

fn one_based(start: usize, end: usize) -> u64 {
    if end <= start || start == usize::MAX {
        // An empty side points at the line before the change.
        start.min(end) as u64
    } else {
        start as u64 + 1
    }
}

/// `line` without its line ending, cut to at most `max_chars` characters
/// including the `…` that marks a cut. The flag says whether it was cut.
fn cut_line(line: &str, max_chars: usize) -> (String, bool) {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.chars().count() <= max_chars {
        return (line.to_string(), false);
    }
    let end = line
        .char_indices()
        .nth(max_chars.saturating_sub(1))
        .map_or(line.len(), |(end, _)| end);
    (format!("{}…", &line[..end]), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_has_no_hunks() {
        let h = unified_hunks("a\nb\n", "a\nb\n", 400, 512);
        assert!(h.hunks.is_empty());
        assert_eq!((h.added, h.removed, h.truncated), (0, 0, false));
    }

    #[test]
    fn a_change_in_the_middle_keeps_three_lines_of_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
        let new = "1\n2\n3\n4\nFIVE\n6\n7\n8\n9\n";
        let h = unified_hunks(old, new, 400, 512);
        assert_eq!(h.hunks.len(), 1);
        let hunk = &h.hunks[0];
        assert_eq!((hunk.old_start, hunk.old_lines), (2, 7));
        assert_eq!((hunk.new_start, hunk.new_lines), (2, 7));
        assert_eq!(hunk.lines.len(), 8);
    }

    #[test]
    fn creating_a_file_points_the_old_side_at_zero() {
        let h = unified_hunks("", "a\nb\n", 400, 512);
        let hunk = &h.hunks[0];
        assert_eq!((hunk.old_start, hunk.old_lines), (0, 0));
        assert_eq!((hunk.new_start, hunk.new_lines), (1, 2));
        assert_eq!(h.added, 2);
    }

    #[test]
    fn a_missing_trailing_newline_does_not_leak_into_the_text() {
        let h = unified_hunks("a", "b", 400, 512);
        let texts: Vec<&str> = h.hunks[0].lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["a", "b"]);
    }

    #[test]
    fn lines_beyond_the_budget_are_cut_but_still_counted() {
        let new: String = (0..10).map(|i| format!("{i}\n")).collect();
        let h = unified_hunks("", &new, 4, 512);
        assert_eq!(h.hunks[0].lines.len(), 4);
        assert_eq!(h.added, 10);
        assert!(h.truncated);
    }

    #[test]
    fn long_lines_are_cut_on_a_char_boundary() {
        let h = unified_hunks("", &"é".repeat(600), 400, 512);
        let text = &h.hunks[0].lines[0].text;
        assert_eq!(text.chars().count(), 512);
        assert!(text.ends_with('…'));
        assert!(h.truncated);
    }
}
