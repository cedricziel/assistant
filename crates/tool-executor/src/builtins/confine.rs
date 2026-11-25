//! Workspace-root confinement shared by the file tools.

use std::path::PathBuf;

use assistant_core::permission::WorkspaceRoots;
use assistant_core::{ToolOutput, types::conversation::ExecutionContext};

/// The workspace roots for this call: the persona workspace plus `extra_roots`.
pub(crate) fn roots(ctx: &ExecutionContext, extra_roots: &[String]) -> WorkspaceRoots {
    WorkspaceRoots::for_agent(&ctx.agent_id, extra_roots)
}

/// Resolve `raw` (symlinks and `..` included) and require it to lie inside the
/// roots. The returned path is the resolved one; use it for the actual I/O.
pub(crate) fn confine(
    ctx: &ExecutionContext,
    extra_roots: &[String],
    raw: &str,
) -> Result<PathBuf, ToolOutput> {
    roots(ctx, extra_roots).confine(raw).map_err(|outside| {
        ToolOutput::error(format!(
            "{outside}; file tools are confined to the workspace roots"
        ))
    })
}
