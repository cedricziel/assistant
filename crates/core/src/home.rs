//! Expansion of `~` in paths users write into config files.
//!
//! This is the only library code that reads the home directory. The install
//! root is never derived here: each entry point resolves it and passes it
//! down.

use std::path::PathBuf;

/// Expand a bare `~` or a leading `~/` to the current user's home directory.
/// Other paths, and every path when the home directory is unknown, pass
/// through unchanged.
pub fn expand_tilde(s: &str) -> PathBuf {
    let rest = match s.strip_prefix('~') {
        Some("") => "",
        Some(rest) => match rest.strip_prefix('/') {
            Some(rest) => rest,
            None => return PathBuf::from(s),
        },
        None => return PathBuf::from(s),
    };
    match dirs::home_dir() {
        Some(home) if rest.is_empty() => home,
        Some(home) => home.join(rest),
        None => PathBuf::from(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_paths_without_a_leading_tilde_alone() {
        assert_eq!(expand_tilde("/etc/hosts"), PathBuf::from("/etc/hosts"));
        assert_eq!(expand_tilde("~user/x"), PathBuf::from("~user/x"));
        assert_eq!(expand_tilde("a/~/b"), PathBuf::from("a/~/b"));
    }

    #[test]
    fn expands_bare_and_leading_tilde_to_the_same_home() {
        let home = expand_tilde("~");
        assert_eq!(expand_tilde("~/notes.md"), home.join("notes.md"));
    }
}
