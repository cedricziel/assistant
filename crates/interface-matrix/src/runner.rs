//! Matrix interface runner.
//!
//! Thin entry point — all dispatch logic lives in [`ChannelRunner`].

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};

use crate::adapter::MatrixAdapter;
use crate::client::MatrixClient;
use crate::config::MatrixConfigExt;

pub use assistant_core::MatrixConfig;

/// Matrix interface runner.
pub struct MatrixInterface {
    config: MatrixConfig,
    orchestrator: Arc<Orchestrator>,
}

impl MatrixInterface {
    pub fn new(config: MatrixConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let homeserver_url = self.config.resolved_homeserver_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Matrix homeserver URL configured. Set homeserver_url in [matrix] config \
                 or MATRIX_HOMESERVER_URL env var."
            )
        })?;

        // Authenticate.
        let client = Arc::new(if let Some(token) = self.config.resolved_access_token() {
            let username = self.config.resolved_username().ok_or_else(|| {
                anyhow::anyhow!(
                    "access_token is set but username is missing. \
                     Set username in [matrix] config or MATRIX_USERNAME env var."
                )
            })?;
            tracing::info!("Matrix: using access token auth");
            MatrixClient::new_with_token(&homeserver_url, token, username)?
        } else {
            let username = self.config.resolved_username().ok_or_else(|| {
                anyhow::anyhow!(
                    "No Matrix credentials. Set access_token or username+password in [matrix] config."
                )
            })?;
            let password = self
                .config
                .resolved_password()
                .ok_or_else(|| anyhow::anyhow!("username is set but password is missing."))?;
            tracing::info!(username = %username, "Matrix: logging in with password");
            MatrixClient::login(&homeserver_url, &username, &password).await?
        });

        tracing::info!(user_id = %client.user_id, "Matrix: authenticated");

        let adapter = Arc::new(MatrixAdapter::new(
            client,
            self.config.allowed_rooms.clone(),
            self.config.allowed_users.clone(),
        ));

        ChannelRunner::new(adapter, self.orchestrator.clone())
            .run()
            .await
    }
}

#[async_trait::async_trait]
impl InterfaceRunner for MatrixInterface {
    async fn run(&self) -> Result<()> {
        MatrixInterface::run(self).await
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::MatrixConfig;

    #[test]
    fn allowlist_room_empty_accepts_all() {
        let cfg = MatrixConfig {
            allowed_rooms: vec![],
            ..Default::default()
        };
        let room_id = "!abc:example.com".to_string();
        let blocked = !cfg.allowed_rooms.is_empty() && !cfg.allowed_rooms.contains(&room_id);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_room_non_empty_blocks_unknown() {
        let cfg = MatrixConfig {
            allowed_rooms: vec!["!known:example.com".to_string()],
            ..Default::default()
        };
        let blocked = !cfg.allowed_rooms.is_empty()
            && !cfg
                .allowed_rooms
                .contains(&"!other:example.com".to_string());
        assert!(blocked);
    }
}
