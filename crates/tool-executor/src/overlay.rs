//! [`ToolOverlay`]: tools that exist for one turn only.
//!
//! The executor's registry is shared by every turn of a space, so a tool that
//! depends on who is asking (a user's own integration, say) cannot live there.
//! A turn builds an overlay, owns it, and passes it to
//! [`ToolExecutor::to_specs_with`](crate::ToolExecutor::to_specs_with) and
//! [`ToolExecutor::execute_with`](crate::ToolExecutor::execute_with). Nothing
//! is shared between overlays, so two turns cannot see each other's tools.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::{ToolHandler, ToolSpec};

/// Extra tool handlers visible to the turn that owns the overlay.
#[derive(Clone, Default)]
pub struct ToolOverlay {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl ToolOverlay {
    /// A later handler replaces an earlier one with the same name.
    pub fn new(handlers: Vec<Arc<dyn ToolHandler>>) -> Self {
        Self {
            handlers: handlers
                .into_iter()
                .map(|handler| (handler.name().to_owned(), handler))
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn ToolHandler>> {
        self.handlers.get(name)
    }

    pub fn handlers(&self) -> impl Iterator<Item = &Arc<dyn ToolHandler>> {
        self.handlers.values()
    }

    pub(crate) fn specs(&self) -> impl Iterator<Item = ToolSpec> + '_ {
        self.handlers.values().map(spec_of)
    }
}

pub(crate) fn spec_of(handler: &Arc<dyn ToolHandler>) -> ToolSpec {
    ToolSpec {
        name: handler.name().to_string(),
        description: handler.description().to_string(),
        params_schema: handler.params_schema(),
        is_mutating: handler.is_mutating(),
        requires_confirmation: handler.requires_confirmation(),
    }
}
