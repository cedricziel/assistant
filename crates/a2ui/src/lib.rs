//! A2UI v1.0 wire types and the assistant component catalog.
//!
//! The server builds A2UI messages from a validated surface draft; the model
//! never authors envelopes. See `docs/adr/adr-0019-generative-ui-a2ui.md`.

pub mod catalog;
pub mod messages;
mod safety;
pub mod validate;

pub use catalog::{CATALOG_ID, Catalog, ValidationIssue};
pub use messages::{
    Component, CreateSurface, DataUpdate, Message, MessageBody, ProtocolVersion, SurfaceDraft,
    SurfaceMessages, SurfacePatch, UpdateComponents, UpdateDataModel,
};
pub use validate::{UrlPolicy, ValidSurface, validate};
