//! Path resolution and workspace-root confinement shared by the permission
//! rules and the file tools.

use std::path::{Component, Path, PathBuf};

use crate::context::{default_workspace_dir, runtime_workspace_dir};
use crate::memory::expand_tilde;

const MAX_SYMLINK_HOPS: usize = 40;

/// Resolve `raw` to the location it really refers to.
///
/// Expands `~/`, makes relative paths absolute against `base` (or the current
/// directory), normalises `.` and `..` *physically* (a `..` after a symlink
/// pops the symlink target, not the link), and resolves symlinks. Components
/// that do not exist yet are kept lexically, so the result is usable for
/// files about to be created.
pub fn resolve_real_path(raw: &str, base: Option<&Path>) -> PathBuf {
    let expanded = expand_tilde(raw);
    let start = if expanded.is_absolute() {
        PathBuf::new()
    } else {
        base.map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .map(|b| resolve_components(&b, PathBuf::new(), &mut 0))
            .unwrap_or_default()
    };
    resolve_components(&expanded, start, &mut 0)
}

fn resolve_components(path: &Path, mut out: PathBuf, hops: &mut usize) -> PathBuf {
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
                    && *hops < MAX_SYMLINK_HOPS
                    && let Ok(target) = std::fs::read_link(&out)
                {
                    *hops += 1;
                    out.pop();
                    out = resolve_components(&target, out, hops);
                }
            }
        }
    }
    out
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
            let p = resolve_real_path(r.as_ref(), None);
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
        all.push(
            default_workspace_dir(agent_id)
                .to_string_lossy()
                .into_owned(),
        );
        all.extend(extra.iter().map(|s| s.as_ref().to_string()));
        Self::new(all)
    }

    /// The resolved roots.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Resolve `raw` and return it if it lies inside a root.
    pub fn confine(&self, raw: &str) -> Result<PathBuf, PathOutsideRoots> {
        let real = resolve_real_path(raw, None);
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
        let got = resolve_real_path(&format!("{}/a/../b/new.txt", root.display()), None);
        assert_eq!(got, root.join("b/new.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn resolves_symlink_to_real_target() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        std::fs::create_dir(root.join("real")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
        let got = resolve_real_path(&format!("{}/link/f.txt", root.display()), None);
        assert_eq!(got, root.join("real/f.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn dotdot_after_symlink_pops_the_target_not_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        std::fs::create_dir_all(root.join("deep/inner")).unwrap();
        std::os::unix::fs::symlink(root.join("deep/inner"), root.join("link")).unwrap();
        let got = resolve_real_path(&format!("{}/link/../x", root.display()), None);
        assert_eq!(got, root.join("deep/x"));
    }

    #[test]
    fn relative_paths_resolve_against_base() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(dir.path());
        assert_eq!(resolve_real_path("n.txt", Some(&root)), root.join("n.txt"));
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
}
