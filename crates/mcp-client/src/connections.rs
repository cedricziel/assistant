//! [`IntegrationConnections`]: the process-wide pool of connections to
//! integration MCP servers.
//!
//! A connection belongs to one `(integration, subject)` pair, so a per-user
//! integration opens one session per user and never sends one user's
//! credential on another's behalf. Connections open lazily on the first turn
//! that needs them, keep their `tools/list` cached, and are dropped when idle
//! or when the integration or credential changes.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use http::{HeaderName, HeaderValue};
use rmcp::ServiceExt;
use rmcp::model::Tool;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, warn};

use assistant_core::clock::Clock;
use assistant_core::identity::UserId;
use assistant_core::integrations::{
    CredentialKind, CredentialStatus, EffectiveIntegration, Integration, IntegrationAuthKind,
    IntegrationCredentialStore, IntegrationHealthStore, Sealed, SecretBox, StoredCredential,
    credential_aad,
};
use assistant_core::secret::Secret;

use crate::client::McpClient;

/// The longest error text kept in the health row.
const MAX_ERROR_LEN: usize = 300;

/// Timing knobs of the pool.
#[derive(Clone, Debug)]
pub struct ConnectionsConfig {
    /// A connection unused for this long is dropped.
    pub idle_timeout: Duration,
    /// How long opening a connection and listing its tools may take.
    pub connect_timeout: Duration,
    /// After a failed attempt, how long to answer from that failure instead of
    /// trying again.
    pub retry_after: Duration,
    /// How long a cached `tools/list` is trusted on a live connection.
    pub tools_ttl: Duration,
}

impl Default for ConnectionsConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(10 * 60),
            connect_timeout: Duration::from_secs(5),
            retry_after: Duration::from_secs(30),
            tools_ttl: Duration::from_secs(5 * 60),
        }
    }
}

/// Why an integration could not be connected for a caller.
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectError {
    /// The credential is missing, unusable or cannot be decrypted.
    Credential(String),
    /// The server did not answer in time, refused, or spoke something else.
    Unreachable(String),
    /// Reading the stores failed.
    Internal(String),
}

impl ConnectError {
    /// The reason alone, without the kind prefix [`Display`](fmt::Display) adds.
    pub fn message(&self) -> &str {
        match self {
            Self::Credential(m) | Self::Unreachable(m) | Self::Internal(m) => m,
        }
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credential(m) => write!(f, "credential unusable: {m}"),
            Self::Unreachable(m) => write!(f, "unreachable: {m}"),
            Self::Internal(m) => write!(f, "internal error: {m}"),
        }
    }
}

impl std::error::Error for ConnectError {}

/// A live connection and the tools it offered.
#[derive(Clone)]
pub struct Connected {
    pub client: Arc<McpClient>,
    pub tools: Arc<[Tool]>,
}

/// What a connection was built from; a change to either invalidates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Versions {
    integration: DateTime<Utc>,
    credential: Option<DateTime<Utc>>,
}

struct Live {
    connected: Connected,
    versions: Versions,
    last_used: Instant,
    listed_at: Instant,
}

struct Failure {
    versions: Versions,
    at: Instant,
    error: String,
}

#[derive(Default)]
struct Slot {
    live: Option<Live>,
    failure: Option<Failure>,
}

type Key = (String, Option<UserId>);

pub struct IntegrationConnections {
    credentials: Arc<dyn IntegrationCredentialStore>,
    health: Arc<dyn IntegrationHealthStore>,
    secrets: Arc<SecretBox>,
    clock: Arc<dyn Clock>,
    config: ConnectionsConfig,
    slots: Mutex<HashMap<Key, Arc<AsyncMutex<Slot>>>>,
}

impl IntegrationConnections {
    pub fn new(
        credentials: Arc<dyn IntegrationCredentialStore>,
        health: Arc<dyn IntegrationHealthStore>,
        secrets: Arc<SecretBox>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            credentials,
            health,
            secrets,
            clock,
            config: ConnectionsConfig::default(),
            slots: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_config(mut self, config: ConnectionsConfig) -> Self {
        self.config = config;
        self
    }

    /// The connection of `effective`, opened if needed.
    ///
    /// A failure is recorded as the integration's health and answered again
    /// without a new attempt for [`ConnectionsConfig::retry_after`], so a dead
    /// server costs one connect timeout, not one per turn.
    pub async fn connect(
        &self,
        effective: &EffectiveIntegration,
    ) -> Result<Connected, ConnectError> {
        self.evict_idle().await;
        let integration = &effective.integration;
        let subject = effective.credential_subject.as_ref();
        let stored = self.stored_credential(integration, subject).await?;
        let versions = Versions {
            integration: integration.updated_at,
            credential: stored.as_ref().map(|c| c.updated_at),
        };

        let slot = self.slot(integration, subject);
        let mut slot = slot.lock().await;
        let now = self.clock.now_instant();

        if let Some(live) = &mut slot.live {
            let fresh = live.versions == versions
                && live.connected.client.is_connected()
                && !self.idle(live, now);
            if fresh && now.saturating_duration_since(live.listed_at) >= self.config.tools_ttl {
                match live.connected.client.list_tools().await {
                    Ok(tools) => {
                        live.connected.tools = tools.into();
                        live.listed_at = now;
                    }
                    Err(e) => {
                        debug!(integration = %integration.id, error = %e, "tools refresh failed; reconnecting");
                        slot.live = None;
                    }
                }
            } else if !fresh {
                slot.live = None;
            }
        }
        if let Some(live) = &mut slot.live {
            live.last_used = now;
            return Ok(live.connected.clone());
        }

        if let Some(failure) = &slot.failure {
            let recent = now.saturating_duration_since(failure.at) < self.config.retry_after;
            if failure.versions == versions && recent {
                return Err(ConnectError::Unreachable(failure.error.clone()));
            }
        }

        let header = self.auth_header(integration, subject, stored.as_ref())?;
        match open(
            &integration.slug,
            &integration.connect_url,
            header,
            self.config.connect_timeout,
        )
        .await
        {
            Ok((client, tools)) => {
                self.record_connected(integration, subject, tools.len())
                    .await;
                let connected = Connected {
                    client: Arc::new(client),
                    tools: tools.into(),
                };
                slot.failure = None;
                slot.live = Some(Live {
                    connected: connected.clone(),
                    versions,
                    last_used: now,
                    listed_at: now,
                });
                Ok(connected)
            }
            Err(error) => {
                self.record_failed(integration, subject, &error).await;
                slot.failure = Some(Failure {
                    versions,
                    at: now,
                    error: error_text(&error),
                });
                Err(error)
            }
        }
    }

    /// Connect once without keeping anything, and record the outcome as the
    /// integration's health. For checks that must not leave a session open,
    /// like the `test` endpoint.
    pub async fn probe(&self, effective: &EffectiveIntegration) -> Result<Vec<Tool>, ConnectError> {
        let integration = &effective.integration;
        let subject = effective.credential_subject.as_ref();
        let stored = self.stored_credential(integration, subject).await?;
        let header = self.auth_header(integration, subject, stored.as_ref())?;
        match open(
            &integration.slug,
            &integration.connect_url,
            header,
            self.config.connect_timeout,
        )
        .await
        {
            Ok((_client, tools)) => {
                self.record_connected(integration, subject, tools.len())
                    .await;
                Ok(tools)
            }
            Err(error) => {
                self.record_failed(integration, subject, &error).await;
                Err(error)
            }
        }
    }

    /// Drop every connection that has been idle for the idle timeout.
    pub async fn evict_idle(&self) {
        let now = self.clock.now_instant();
        for slot in self.all_slots() {
            // A slot in use is not idle.
            let Ok(mut slot) = slot.try_lock() else {
                continue;
            };
            if slot.live.as_ref().is_some_and(|live| self.idle(live, now)) {
                slot.live = None;
            }
        }
    }

    /// Connections currently open.
    pub async fn connection_count(&self) -> usize {
        let mut open = 0;
        for slot in self.all_slots() {
            if slot.lock().await.live.is_some() {
                open += 1;
            }
        }
        open
    }

    fn all_slots(&self) -> Vec<Arc<AsyncMutex<Slot>>> {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    fn idle(&self, live: &Live, now: Instant) -> bool {
        now.saturating_duration_since(live.last_used) > self.config.idle_timeout
    }

    fn slot(&self, integration: &Integration, subject: Option<&UserId>) -> Arc<AsyncMutex<Slot>> {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry((integration.id.clone(), subject.cloned()))
            .or_default()
            .clone()
    }

    /// The caller's stored credential, or `None` for an integration that needs
    /// none.
    async fn stored_credential(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
    ) -> Result<Option<StoredCredential>, ConnectError> {
        if integration.auth_kind == IntegrationAuthKind::None {
            return Ok(None);
        }
        let stored = self
            .credentials
            .get(&integration.id, subject)
            .await
            .map_err(|e| ConnectError::Internal(format!("loading the credential: {e:#}")))?;
        match stored {
            None => Err(ConnectError::Credential("no credential is stored".into())),
            Some(c) if c.status != CredentialStatus::Ready => Err(ConnectError::Credential(
                "the stored credential is marked unusable".into(),
            )),
            Some(c) => Ok(Some(c)),
        }
    }

    /// The header the integration's template forms from its decrypted credential.
    fn auth_header(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
        stored: Option<&StoredCredential>,
    ) -> Result<Option<(HeaderName, HeaderValue)>, ConnectError> {
        let Some(stored) = stored else {
            return Ok(None);
        };
        if stored.kind != CredentialKind::Header {
            return Err(ConnectError::Credential(
                "OAuth credentials are not supported yet".into(),
            ));
        }
        let aad = credential_aad(
            &integration.id,
            subject.map(|user| user.0.as_str()),
            CredentialKind::Header.as_str(),
        );
        let sealed = Sealed {
            ciphertext: stored.ciphertext.clone(),
            nonce: stored.nonce.clone(),
            key_id: stored.key_id.clone(),
        };
        let secret = self
            .secrets
            .open(&sealed, aad.as_bytes())
            .map_err(|e| ConnectError::Credential(format!("{e}")))?;
        header_for(integration, &secret).map(Some)
    }

    async fn record_connected(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
        tools: usize,
    ) {
        let tools = u32::try_from(tools).unwrap_or(u32::MAX);
        if let Err(e) = self
            .health
            .record_connected(&integration.id, subject, tools, self.clock.now())
            .await
        {
            warn!(integration = %integration.id, error = %e, "could not record integration health");
        }
    }

    async fn record_failed(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
        error: &ConnectError,
    ) {
        warn!(integration = %integration.id, %error, "integration connection failed");
        if let Err(e) = self
            .health
            .record_failed(
                &integration.id,
                subject,
                &error_text(error),
                self.clock.now(),
            )
            .await
        {
            warn!(integration = %integration.id, error = %e, "could not record integration health");
        }
    }
}

/// The error as stored and shown: bounded, and free of anything but what the
/// transport reported.
fn error_text(error: &ConnectError) -> String {
    error.message().chars().take(MAX_ERROR_LEN).collect()
}

fn header_for(
    integration: &Integration,
    secret: &Secret,
) -> Result<(HeaderName, HeaderValue), ConnectError> {
    let name = integration
        .header_name
        .as_deref()
        .ok_or_else(|| ConnectError::Credential("the integration has no header name".into()))?;
    let name = HeaderName::try_from(name)
        .map_err(|_| ConnectError::Credential("the header name is invalid".into()))?;
    let value = format!(
        "{}{}",
        integration.header_value_prefix.as_deref().unwrap_or(""),
        secret.expose()
    );
    let mut value = HeaderValue::try_from(value).map_err(|_| {
        ConnectError::Credential("the credential is not a valid header value".into())
    })?;
    value.set_sensitive(true);
    Ok((name, value))
}

/// Open a Streamable HTTP session and list its tools within `timeout`.
async fn open(
    name: &str,
    url: &str,
    header: Option<(HeaderName, HeaderValue)>,
    timeout: Duration,
) -> Result<(McpClient, Vec<Tool>), ConnectError> {
    let attempt = async {
        let mut config = StreamableHttpClientTransportConfig::with_uri(url);
        if let Some((header_name, value)) = header {
            config = config.custom_headers(HashMap::from([(header_name, value)]));
        }
        let transport = StreamableHttpClientTransport::<reqwest::Client>::from_config(config);
        let service = ()
            .serve(transport)
            .await
            .map_err(|e| anyhow!("initialize handshake failed: {e}"))?;
        let client = McpClient::new(name, service);
        let tools = client.list_tools().await?;
        anyhow::Ok((client, tools))
    };
    match tokio::time::timeout(timeout, attempt).await {
        Ok(Ok(opened)) => Ok(opened),
        Ok(Err(e)) => Err(ConnectError::Unreachable(format!("{e:#}"))),
        Err(_) => Err(ConnectError::Unreachable(format!(
            "connection timed out after {timeout:?}"
        ))),
    }
}
