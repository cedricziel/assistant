//! Matrix interface tools.
//!
//! Extension tools registered for Matrix turns.  Currently a stub — v1
//! does not expose any ambient Matrix-specific tools.

use std::sync::Arc;

use assistant_core::ToolHandler;

/// Build per-turn Matrix extension tools.
///
/// Returns an empty list for v1.  Future versions may add a `matrix-post`
/// ambient tool for proactive messaging, following the Slack pattern.
pub fn build_matrix_tools() -> Vec<Arc<dyn ToolHandler>> {
    vec![]
}
