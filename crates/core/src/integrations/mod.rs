//! Integrations: the cached integrations.sh catalog, credential encryption, the
//! domain model and its stores.

pub mod catalog;
pub mod effective;
pub mod memory;
pub mod model;
pub mod secret;

pub use effective::{
    Availability, EffectiveIntegration, EffectiveIntegrations, EvaluatedIntegration,
};
pub use memory::{
    InMemoryIntegrationCredentialStore, InMemoryIntegrationHealthStore, InMemoryIntegrationStore,
};
pub use model::*;
pub use secret::{Sealed, SecretBox, SecretBoxError, credential_aad};
