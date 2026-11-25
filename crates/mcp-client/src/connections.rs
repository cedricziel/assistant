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
    IntegrationCredentialStore, IntegrationHealthStore, IntegrationStore, Sealed, SecretBox,
    StoredCredential, credential_aad,
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
    /// How often the pool drops idle connections on its own.
    pub sweep_interval: Duration,
    /// The shortest time between two [`probe`](IntegrationConnections::probe)s
    /// of the same integration and subject.
    pub probe_interval: Duration,
    /// Send credentials over plain `http://` too. Off by default: a credential
    /// would travel in the clear. For tests against local servers only.
    pub allow_plain_http: bool,
}

impl Default for ConnectionsConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(10 * 60),
            connect_timeout: Duration::from_secs(5),
            retry_after: Duration::from_secs(30),
            tools_ttl: Duration::from_secs(5 * 60),
            sweep_interval: Duration::from_secs(60),
            probe_interval: Duration::from_secs(5),
            allow_plain_http: false,
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
    /// A probe came too soon after the last one for the same integration and
    /// subject.
    Throttled { retry_after: Duration },
    /// The integration changed after the caller read it; its credential no
    /// longer belongs with the URL the caller holds. Retry with a fresh read.
    Changed,
}

impl ConnectError {
    /// The reason alone, without the kind prefix [`Display`](fmt::Display) adds.
    pub fn message(&self) -> String {
        match self {
            Self::Credential(m) | Self::Unreachable(m) | Self::Internal(m) => m.clone(),
            Self::Throttled { retry_after } => {
                format!(
                    "tested too recently; retry in {}s",
                    retry_after.as_secs().max(1)
                )
            }
            Self::Changed => "the integration changed; try again".into(),
        }
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credential(m) => write!(f, "credential unusable: {m}"),
            Self::Unreachable(m) => write!(f, "unreachable: {m}"),
            Self::Internal(m) => write!(f, "internal error: {m}"),
            Self::Throttled { .. } | Self::Changed => f.write_str(&self.message()),
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

/// What the background sweep needs: nothing that keeps the pool's stores alive.
struct Shared {
    clock: Arc<dyn Clock>,
    config: ConnectionsConfig,
    slots: Mutex<HashMap<Key, Arc<AsyncMutex<Slot>>>>,
    /// When each integration and subject was last probed.
    probes: Mutex<HashMap<Key, Instant>>,
}

/// Stops the sweep when the pool is dropped.
struct Sweeper(tokio::task::JoinHandle<()>);

impl Drop for Sweeper {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct IntegrationConnections {
    integrations: Arc<dyn IntegrationStore>,
    credentials: Arc<dyn IntegrationCredentialStore>,
    health: Arc<dyn IntegrationHealthStore>,
    secrets: Arc<SecretBox>,
    shared: Arc<Shared>,
    _sweeper: Option<Sweeper>,
}

impl IntegrationConnections {
    pub fn new(
        integrations: Arc<dyn IntegrationStore>,
        credentials: Arc<dyn IntegrationCredentialStore>,
        health: Arc<dyn IntegrationHealthStore>,
        secrets: Arc<SecretBox>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self::assemble(
            integrations,
            credentials,
            health,
            secrets,
            clock,
            ConnectionsConfig::default(),
        )
    }

    pub fn with_config(self, config: ConnectionsConfig) -> Self {
        Self::assemble(
            self.integrations.clone(),
            self.credentials.clone(),
            self.health.clone(),
            self.secrets.clone(),
            self.shared.clock.clone(),
            config,
        )
    }

    /// Builds the pool and, when a Tokio runtime is running, starts the sweep
    /// that drops idle connections. The sweep ends with the pool.
    fn assemble(
        integrations: Arc<dyn IntegrationStore>,
        credentials: Arc<dyn IntegrationCredentialStore>,
        health: Arc<dyn IntegrationHealthStore>,
        secrets: Arc<SecretBox>,
        clock: Arc<dyn Clock>,
        config: ConnectionsConfig,
    ) -> Self {
        let interval = config.sweep_interval;
        let shared = Arc::new(Shared {
            clock,
            config,
            slots: Mutex::new(HashMap::new()),
            probes: Mutex::new(HashMap::new()),
        });
        let sweeper = tokio::runtime::Handle::try_current().ok().map(|runtime| {
            Sweeper(runtime.spawn(sweep_periodically(Arc::downgrade(&shared), interval)))
        });
        Self {
            integrations,
            credentials,
            health,
            secrets,
            shared,
            _sweeper: sweeper,
        }
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
        let integration = &effective.integration;
        let subject = effective.credential_subject.as_ref();

        // The credential is read under the slot lock: a read taken before it
        // could be replaced by the time the lock is won, and the stale value
        // would then be cached.
        let slot = self.slot(integration, subject);
        let mut slot = slot.lock().await;
        let stored = match self.stored_credential(integration, subject).await {
            Ok(stored) => stored,
            Err(error) => {
                // A revoked or missing credential must not keep a session open.
                slot.live = None;
                return Err(error);
            }
        };
        self.ensure_unchanged(integration, stored.as_ref()).await?;
        let versions = Versions {
            integration: integration.updated_at,
            credential: stored.as_ref().map(|c| c.updated_at),
        };
        let now = self.shared.clock.now_instant();
        let config = &self.shared.config;

        if let Some(live) = &mut slot.live {
            let fresh = live.versions == versions
                && live.connected.client.is_connected()
                && !self.shared.idle(live, now);
            if fresh && now.saturating_duration_since(live.listed_at) >= config.tools_ttl {
                match tokio::time::timeout(
                    config.connect_timeout,
                    live.connected.client.list_tools(),
                )
                .await
                {
                    Ok(Ok(tools)) => {
                        live.connected.tools = tools.into();
                        live.listed_at = now;
                    }
                    Ok(Err(e)) => {
                        let kind = classify(&format!("{e:#}"));
                        debug!(integration = %integration.id, %kind, "tools refresh failed; reconnecting");
                        slot.live = None;
                    }
                    Err(_) => {
                        // A session that stops answering is skipped, not waited on again.
                        slot.live = None;
                        let error = ConnectError::Unreachable(TIMEOUT.into());
                        return Err(self
                            .remember_failure(&mut slot, integration, subject, versions, error)
                            .await);
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
            let recent = now.saturating_duration_since(failure.at) < config.retry_after;
            if failure.versions == versions && recent {
                return Err(ConnectError::Unreachable(failure.error.clone()));
            }
        }

        let header = self.auth_header(integration, subject, stored.as_ref())?;
        match open(
            &integration.slug,
            &integration.connect_url,
            header,
            config.connect_timeout,
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
            Err(error) => Err(self
                .remember_failure(&mut slot, integration, subject, versions, error)
                .await),
        }
    }

    /// Record `error` as the integration's health and remember it, so the next
    /// turns are answered from it for a while. The pause counts from now, not
    /// from when the failed attempt began.
    async fn remember_failure(
        &self,
        slot: &mut Slot,
        integration: &Integration,
        subject: Option<&UserId>,
        versions: Versions,
        error: ConnectError,
    ) -> ConnectError {
        self.record_failed(integration, subject, &error).await;
        slot.failure = Some(Failure {
            versions,
            at: self.shared.clock.now_instant(),
            error: error_text(&error),
        });
        error
    }

    /// Connect once without keeping anything, and record the outcome as the
    /// integration's health. For checks that must not leave a session open,
    /// like the `test` endpoint.
    ///
    /// A probe skips the failure pause, so each integration and subject may be
    /// probed only once per [`ConnectionsConfig::probe_interval`]; sooner, it
    /// fails with [`ConnectError::Throttled`] without contacting the server.
    pub async fn probe(&self, effective: &EffectiveIntegration) -> Result<Vec<Tool>, ConnectError> {
        let integration = &effective.integration;
        let subject = effective.credential_subject.as_ref();
        self.shared.reserve_probe(integration, subject)?;
        let prepared = async {
            let stored = self.stored_credential(integration, subject).await?;
            self.ensure_unchanged(integration, stored.as_ref()).await?;
            self.auth_header(integration, subject, stored.as_ref())
        }
        .await;
        let header = match prepared {
            Ok(header) => header,
            Err(error) => {
                // A credential that cannot be used replaces any healthy record.
                if matches!(error, ConnectError::Credential(_)) {
                    self.record_failed(integration, subject, &error).await;
                }
                return Err(error);
            }
        };
        match open(
            &integration.slug,
            &integration.connect_url,
            header,
            self.shared.config.connect_timeout,
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

    /// Drop every connection that has been idle for the idle timeout, forget
    /// failures that no longer matter, and remove the entries left empty.
    pub async fn evict_idle(&self) {
        self.shared.sweep().await;
    }

    /// Entries the pool keeps: open connections and remembered failures.
    pub fn tracked(&self) -> usize {
        self.shared
            .slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Connections currently open.
    pub async fn connection_count(&self) -> usize {
        let mut open = 0;
        for slot in self.shared.all_slots() {
            if slot.lock().await.live.is_some() {
                open += 1;
            }
        }
        open
    }

    fn slot(&self, integration: &Integration, subject: Option<&UserId>) -> Arc<AsyncMutex<Slot>> {
        self.shared
            .slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry((integration.id.clone(), subject.cloned()))
            .or_default()
            .clone()
    }

    /// Refuse to use `credential` with `snapshot`'s URL unless the stored
    /// integration is still the one the caller read.
    ///
    /// The credential is read first and the integration second, so a URL change
    /// (which deletes the old credentials) between the caller's read and now
    /// shows up here instead of pairing the new host's credential with the old
    /// URL.
    async fn ensure_unchanged(
        &self,
        snapshot: &Integration,
        credential: Option<&StoredCredential>,
    ) -> Result<(), ConnectError> {
        if credential.is_none() {
            return Ok(());
        }
        let current = self
            .integrations
            .get(&snapshot.id)
            .await
            .map_err(|e| ConnectError::Internal(format!("loading the integration: {e:#}")))?;
        match current {
            Some(c)
                if c.connect_url == snapshot.connect_url && c.updated_at == snapshot.updated_at =>
            {
                Ok(())
            }
            _ => Err(ConnectError::Changed),
        }
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
        let https = integration
            .connect_url
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
        if !https && !self.shared.config.allow_plain_http {
            return Err(ConnectError::Credential(
                "credentials are only sent over https".into(),
            ));
        }
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
            .record_connected(&integration.id, subject, tools, self.shared.clock.now())
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
                self.shared.clock.now(),
            )
            .await
        {
            warn!(integration = %integration.id, error = %e, "could not record integration health");
        }
    }
}

impl Shared {
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

    /// Claim the right to probe, or say how long until it is allowed.
    fn reserve_probe(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
    ) -> Result<(), ConnectError> {
        let now = self.clock.now_instant();
        let mut probes = self.probes.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (integration.id.clone(), subject.cloned());
        if let Some(last) = probes.get(&key) {
            let since = now.saturating_duration_since(*last);
            if since < self.config.probe_interval {
                return Err(ConnectError::Throttled {
                    retry_after: self.config.probe_interval - since,
                });
            }
        }
        probes.insert(key, now);
        Ok(())
    }

    async fn sweep(&self) {
        let now = self.clock.now_instant();
        for slot in self.all_slots() {
            // A slot in use is not idle.
            let Ok(mut slot) = slot.try_lock() else {
                continue;
            };
            if slot.live.as_ref().is_some_and(|live| self.idle(live, now)) {
                slot.live = None;
            }
            let expired = |failure: &Failure| {
                now.saturating_duration_since(failure.at) >= self.config.retry_after
            };
            if slot.failure.as_ref().is_some_and(expired) {
                slot.failure = None;
            }
        }
        self.probes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, last| now.saturating_duration_since(*last) < self.config.probe_interval);
        // Nobody else holds an entry the map alone references, and entries are
        // only handed out under this lock, so an empty one can go.
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|_, slot| {
                Arc::strong_count(slot) > 1
                    || slot
                        .try_lock()
                        .map_or(true, |s| s.live.is_some() || s.failure.is_some())
            });
    }
}

/// The background sweep: ends when the pool, and so `shared`, is gone.
async fn sweep_periodically(shared: std::sync::Weak<Shared>, interval: Duration) {
    loop {
        tokio::time::sleep(interval).await;
        let Some(shared) = shared.upgrade() else {
            return;
        };
        shared.sweep().await;
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
///
/// Failures are reported by kind (see [`classify`]); the raw transport error
/// goes to the debug log only, so it never reaches an API response.
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
        let service = ().serve(transport).await.map_err(anyhow::Error::new)?;
        let client = McpClient::new(name, service);
        let tools = client.list_tools().await?;
        anyhow::Ok((client, tools))
    };
    match tokio::time::timeout(timeout, attempt).await {
        Ok(Ok(opened)) => Ok(opened),
        Ok(Err(e)) => {
            // Only the kind is logged: an upstream may echo a credential in its error.
            let kind = classify(&format!("{e:#}"));
            debug!(integration = name, %kind, "integration connection failed");
            Err(ConnectError::Unreachable(kind))
        }
        Err(_) => Err(ConnectError::Unreachable(TIMEOUT.into())),
    }
}

const TIMEOUT: &str = "timeout";

/// Reduce a transport error to a kind safe to show: `HTTP <status>`,
/// `timeout`, `connection failed` or `protocol error`.
///
/// rmcp flattens a failed request to "error sending request", which hides
/// whether the cause was a refused connection, DNS or TLS, so those share
/// `connection failed`.
fn classify(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if let Some(status) = http_status(&lower) {
        return format!("HTTP {status}");
    }
    let kind = if lower.contains("timed out") || lower.contains("timeout") {
        TIMEOUT
    } else if lower.contains("error sending request") {
        "connection failed"
    } else {
        "protocol error"
    };
    kind.into()
}

/// The status in `... http 500 ...` (lowercased input), if any.
fn http_status(lower: &str) -> Option<&str> {
    let after = lower.split("http ").nth(1)?;
    let digits = after.get(..3)?;
    digits.bytes().all(|b| b.is_ascii_digit()).then_some(digits)
}
