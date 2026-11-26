//! [`IntegrationToolSource`]: the integration tools one turn may use.
//!
//! At the start of a turn it resolves which integrations the turn's user may
//! use in the turn's space, connects them through the shared
//! [`IntegrationConnections`] pool, and wraps their tools as
//! `mcp__{layer}_{slug}__{tool}`. Nothing it returns is registered anywhere
//! shared.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::warn;

use assistant_core::clock::Clock;
use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::integrations::{
    EffectiveIntegrations, Integration, IntegrationCredentialStore, IntegrationHealthStore,
    IntegrationStore, IntegrationTrust, SecretBox,
};
use assistant_core::mcp_app::{McpAppServer, McpAppServers};
use assistant_core::store::{OrgStore, SpaceStore};
use assistant_core::tool::{ToolHandler, TurnScope, TurnToolSource};

use crate::apps::ClientAppServer;
use crate::bridge::model_visible_handlers;
use crate::connections::IntegrationConnections;

/// The org-level stores a source reads.
pub struct IntegrationStores {
    pub integrations: Arc<dyn IntegrationStore>,
    pub credentials: Arc<dyn IntegrationCredentialStore>,
    pub health: Arc<dyn IntegrationHealthStore>,
    pub orgs: Arc<dyn OrgStore>,
    pub spaces: Arc<dyn SpaceStore>,
}

pub struct IntegrationToolSource {
    effective: Arc<EffectiveIntegrations>,
    connections: Arc<IntegrationConnections>,
    orgs: Arc<dyn OrgStore>,
    spaces: Arc<dyn SpaceStore>,
}

impl IntegrationToolSource {
    pub fn new(
        effective: Arc<EffectiveIntegrations>,
        connections: Arc<IntegrationConnections>,
        orgs: Arc<dyn OrgStore>,
        spaces: Arc<dyn SpaceStore>,
    ) -> Self {
        Self {
            effective,
            connections,
            orgs,
            spaces,
        }
    }

    /// A source with its own connection pool over `stores`. Build one per
    /// process and share it between spaces, so a credential is not connected
    /// once per space.
    pub fn from_stores(
        stores: IntegrationStores,
        secrets: Arc<SecretBox>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let effective = Arc::new(EffectiveIntegrations::new(
            stores.integrations.clone(),
            stores.credentials.clone(),
            secrets.key_id(),
        ));
        let connections = Arc::new(IntegrationConnections::new(
            stores.integrations,
            stores.credentials,
            stores.health,
            secrets,
            clock,
        ));
        Self::new(effective, connections, stores.orgs, stores.spaces)
    }

    /// The ids behind the turn's org and space slugs.
    async fn locate(&self, scope: &TurnScope<'_>) -> Option<(OrgId, SpaceId)> {
        let found = async {
            let org = self.orgs.get_org_by_slug(scope.org_slug).await?;
            let Some(org) = org else {
                return anyhow::Ok(None);
            };
            let space = self
                .spaces
                .list_spaces(&org.id)
                .await?
                .into_iter()
                .find(|space| space.slug == scope.space_slug);
            Ok(space.map(|space| (org.id, space.id)))
        }
        .await;
        match found {
            Ok(found) => found,
            Err(e) => {
                warn!(error = %e, "could not look up the turn's org and space; no integration tools");
                None
            }
        }
    }
}

#[async_trait]
impl TurnToolSource for IntegrationToolSource {
    async fn tools_for_turn(&self, scope: &TurnScope<'_>) -> Vec<Arc<dyn ToolHandler>> {
        let Some((org, space)) = self.locate(scope).await else {
            return Vec::new();
        };
        let effective = match self.effective.resolve(&org, &space, scope.user_id).await {
            Ok(effective) => effective,
            Err(e) => {
                warn!(error = %e, "could not resolve the turn's integrations; none will be offered");
                return Vec::new();
            }
        };

        // Connect in parallel; a broken integration costs its own timeout, not
        // the turn's, and is simply left out.
        let connected = futures::future::join_all(
            effective
                .iter()
                .map(|e| async move { (e, self.connections.connect(e).await) }),
        )
        .await;

        let mut handlers: Vec<Arc<dyn ToolHandler>> = Vec::new();
        for (effective, result) in connected {
            let integration = &effective.integration;
            let connected = match result {
                Ok(connected) => connected,
                Err(error) => {
                    warn!(
                        integration = %integration.id,
                        layer = %integration.layer,
                        %error,
                        "integration skipped for this turn"
                    );
                    continue;
                }
            };
            handlers.extend(
                model_visible_handlers(
                    &namespace(integration),
                    &connected.tools,
                    &connected.client,
                    integration.trust != IntegrationTrust::Trust,
                )
                .map(|handler| Arc::new(handler) as Arc<dyn ToolHandler>),
            );
        }
        handlers
    }
}

/// The server namespace of an integration's tools, as in `mcp__{namespace}__{tool}`.
fn namespace(integration: &Integration) -> String {
    format!("{}_{}", integration.layer, integration.slug)
}

#[async_trait]
impl McpAppServers for IntegrationToolSource {
    /// The integration whose tools are named `server`, connected with the
    /// caller's own credential.
    async fn server(
        &self,
        scope: &TurnScope<'_>,
        server: &str,
    ) -> Result<Option<Arc<dyn McpAppServer>>> {
        let Some((org, space)) = self.locate(scope).await else {
            return Ok(None);
        };
        let effective = self.effective.resolve(&org, &space, scope.user_id).await?;
        let Some(found) = effective
            .iter()
            .find(|e| namespace(&e.integration) == server)
        else {
            return Ok(None);
        };
        let connected = self.connections.connect(found).await?;
        Ok(Some(Arc::new(ClientAppServer::new(
            connected.client,
            connected.tools,
            found.integration.trust != IntegrationTrust::Trust,
        ))))
    }
}
