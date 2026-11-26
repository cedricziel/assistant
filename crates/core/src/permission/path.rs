//! Path resolution and workspace-root confinement shared by the permission
//! rules and the file tools.

use std::path::{Component, Path, PathBuf};

use crate::context::{default_workspace_dir, runtime_install_root, runtime_workspace_dir};
use crate::home::expand_tilde;

const MAX_SYMLINK_HOPS: usize = 40;

/// Resolve `raw` to the location it really refers to.
///
/// Expands `~/`, makes relative paths absolute against `base` (or the current
/// directory), normalises `.` and `..` *physically* (a `..` after a symlink
/// pops the symlink target, not the link), and resolves symlinks. Components
/// that do not exist yet are kept lexically, so the result is usable for
/// files about to be created.
///
/// Fails with [`SymlinkBudgetExhausted`] when resolution follows more than
/// `MAX_SYMLINK_HOPS` links (a cycle or an over-long chain), so callers can
/// refuse the path instead of acting on a half-resolved one.
pub fn resolve_real_path(
    raw: &str,
    base: Option<&Path>,
) -> Result<PathBuf, SymlinkBudgetExhausted> {
    let expanded = expand_tilde(raw);
    let start = if expanded.is_absolute() {
        PathBuf::new()
    } else {
        base.map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .map(|b| resolve_components(&b, PathBuf::new(), &mut 0))
            .transpose()?
            .unwrap_or_default()
    };
    resolve_components(&expanded, start, &mut 0)
}

/// Resolution followed more symlinks than the hop budget allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymlinkBudgetExhausted;

impl std::fmt::Display for SymlinkBudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "too many levels of symbolic links")
    }
}

impl std::error::Error for SymlinkBudgetExhausted {}

fn resolve_components(
    path: &Path,
    mut out: PathBuf,
    hops: &mut usize,
) -> Result<PathBuf, SymlinkBudgetExhausted> {
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => {
                out = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(name) => {
                out.push(name);
                if let Ok(meta) = std::fs::symlink_metadata(&out)
                    && meta.file_type().is_symlink()
                    && let Ok(target) = std::fs::read_link(&out)
                {
                    if *hops >= MAX_SYMLINK_HOPS {
                        return Err(SymlinkBudgetExhausted);
                    }
                    *hops += 1;
                    out.pop();
                    out = resolve_components(&target, out, hops)?;
                }
            }
        }
    }
    Ok(out)
}

/// The directory (or literal path) a glob pattern is rooted at: everything
/// before the first glob metacharacter, cut back to the last `/`. `"."` when
/// nothing literal precedes the first metacharacter.
pub fn glob_literal_dir(pattern: &str) -> String {
    let end = pattern.find(['*', '?', '[', '{']).unwrap_or(pattern.len());
    let literal = &pattern[..end];
    let dir = match literal.rfind('/') {
        Some(i) if end < pattern.len() => &literal[..=i],
        _ => literal,
    };
    if dir.is_empty() {
        ".".to_string()
    } else {
        dir.to_string()
    }
}

/// The set of directories file tools may touch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceRoots {
    roots: Vec<PathBuf>,
}

impl WorkspaceRoots {
    /// Build from explicit root paths (each is resolved).
    pub fn new<I, S>(roots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut resolved: Vec<PathBuf> = Vec::new();
        for r in roots {
            let Ok(p) = resolve_real_path(r.as_ref(), None) else {
                continue;
            };
            if !resolved.contains(&p) {
                resolved.push(p);
            }
        }
        Self { roots: resolved }
    }

    /// Roots for a turn: the active persona workspaces plus `extra`.
    pub fn for_agent<S: AsRef<str>>(agent_id: &str, extra: &[S]) -> Self {
        let mut all: Vec<String> = Vec::new();
        if let Some(w) = runtime_workspace_dir() {
            all.push(w.to_string_lossy().into_owned());
        }
        if let Some(root) = runtime_install_root() {
            all.push(
                default_workspace_dir(&root, agent_id)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        all.extend(extra.iter().map(|s| s.as_ref().to_string()));
        Self::new(all)
    }

    /// The resolved roots.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Resolve `raw` and return it if it lies inside a root.
    pub fn confine(&self, raw: &str) -> Result<PathBuf, PathOutsideRoots> {
        let Ok(real) = resolve_real_path(raw, None) else {
            return Err(PathOutsideRoots {
                path: expand_tilde(raw),
            });
        };
        if self.roots.iter().any(|r| real.starts_with(r)) {
            Ok(real)
        } else {
            Err(PathOutsideRoots { path: real })
        }
    }

    /// Whether an already-resolved path lies inside a root.
    pub fn contains(&self, real: &Path) -> bool {
        self.roots.iter().any(|r| real.starts_with(r))
    }
}

/// A path resolved outside every workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathOutsideRoots {
    pub path: PathBuf,
}

impl std::fmt::Display for PathOutsideRoots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{}' is outside the allowed workspace roots",
            self.path.display()
        )
    }
}

impl std::error::Error for PathOutsideRoots {}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon(p: &Path) -> PathBuf {
        p.canonicalize().unwrap()
    }

    #[test]
    fn resolves_dotdot_lexically_for_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        let got = resolve_real_path(&format!("{}/a/../b/new.txt", root.display()), None).unwrap();
        assert_eq!(got, root.join("b/new.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn resolves_symlink_to_real_target() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        std::fs::create_dir(root.join("real")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
        let got = resolve_real_path(&format!("{}/link/f.txt", root.display()), None).unwrap();
        assert_eq!(got, root.join("real/f.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn dotdot_after_symlink_pops_the_target_not_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        std::fs::create_dir_all(root.join("deep/inner")).unwrap();
        std::os::unix::fs::symlink(root.join("deep/inner"), root.join("link")).unwrap();
        let got = resolve_real_path(&format!("{}/link/../x", root.display()), None).unwrap();
        assert_eq!(got, root.join("deep/x"));
    }

    #[test]
    fn relative_paths_resolve_against_base() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        assert_eq!(
            resolve_real_path("n.txt", Some(&root)).unwrap(),
            root.join("n.txt")
        );
    }

    #[test]
    fn glob_literal_dir_stops_at_the_first_metacharacter() {
        assert_eq!(glob_literal_dir("/a/b/**/*.rs"), "/a/b/");
        assert_eq!(glob_literal_dir("/a/b/file?.txt"), "/a/b/");
        assert_eq!(glob_literal_dir("/a/b/c.txt"), "/a/b/c.txt");
        assert_eq!(glob_literal_dir("**/*.rs"), ".");
        assert_eq!(glob_literal_dir("src/*.rs"), "src/");
    }

    #[test]
    fn confine_accepts_inside_and_rejects_outside() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        let roots = WorkspaceRoots::new([root.to_string_lossy()]);
        assert!(roots.confine(&format!("{}/ok.txt", root.display())).is_ok());
        assert!(roots.confine("/etc/passwd").is_err());
        assert!(
            roots
                .confine(&format!("{}/../escape.txt", root.display()))
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn confine_rejects_symlink_escape() {
        let ws = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let ws_root = canon(ws.path());
        std::fs::write(outside.path().join("secret"), "x").unwrap();
        std::os::unix::fs::symlink(outside.path(), ws_root.join("link")).unwrap();
        let roots = WorkspaceRoots::new([ws_root.to_string_lossy()]);
        assert!(
            roots
                .confine(&format!("{}/link/secret", ws_root.display()))
                .is_err()
        );
    }

    #[test]
    fn sibling_with_shared_prefix_is_outside() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path()).join("ws");
        std::fs::create_dir(&root).unwrap();
        let roots = WorkspaceRoots::new([root.to_string_lossy()]);
        let sibling = format!("{}-evil/x", root.display());
        assert!(roots.confine(&sibling).is_err());
    }

    #[cfg(unix)]
    fn chain(root: &Path, links: usize, end: &Path) {
        for i in 0..links {
            let target = if i + 1 == links {
                end.to_path_buf()
            } else {
                root.join(format!("l{}", i + 1))
            };
            std::os::unix::fs::symlink(target, root.join(format!("l{i}"))).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn confine_refuses_symlink_chain_longer_than_the_hop_budget() {
        let ws = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = canon(ws.path());
        chain(&root, MAX_SYMLINK_HOPS + 2, outside.path());
        let roots = WorkspaceRoots::new([root.to_string_lossy()]);
        assert!(roots.confine(&format!("{}/l0", root.display())).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn confine_accepts_a_short_symlink_chain_inside_the_root() {
        let ws = tempfile::tempdir().unwrap();
        let root = canon(ws.path());
        std::fs::write(root.join("real"), "x").unwrap();
        chain(&root, 3, &root.join("real"));
        let roots = WorkspaceRoots::new([root.to_string_lossy()]);
        assert_eq!(
            roots.confine(&format!("{}/l0", root.display())).unwrap(),
            root.join("real")
        );
    }

    #[cfg(unix)]
    #[test]
    fn confine_refuses_symlink_cycles() {
        let ws = tempfile::tempdir().unwrap();
        let root = canon(ws.path());
        std::os::unix::fs::symlink(root.join("b"), root.join("a")).unwrap();
        std::os::unix::fs::symlink(root.join("a"), root.join("b")).unwrap();
        let roots = WorkspaceRoots::new([root.to_string_lossy()]);
        assert!(roots.confine(&format!("{}/a", root.display())).is_err());
    }
}
