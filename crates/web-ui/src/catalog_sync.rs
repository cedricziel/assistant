//! Keeps a local copy of the integrations.sh catalog.
//!
//! [`CatalogSync`] fetches the configured source at startup and then every
//! 24 hours, using `ETag`/`Last-Modified` for conditional requests. A good
//! response is persisted under `<base>/cache/integrations/` and swapped into
//! an in-memory [`CatalogIndex`]. Any failure leaves the previous copy in
//! place, so a restart serves the last good catalog before the first fetch
//! completes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::header::{ETAG, HeaderName, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};
use tokio::time::MissedTickBehavior;
use tracing::{info, warn};

use assistant_core::clock::{Clock, SystemClock};
use assistant_core::integrations::catalog::CatalogIndex;
use assistant_core::types::features::CatalogConfig;

/// How often the catalog is refreshed.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

const CATALOG_FILE: &str = "catalog.json";
const META_FILE: &str = "meta.json";
/// Upper bound for a catalog body; the real one is under 3 MiB.
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

/// What a refresh did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// A new catalog replaced the cached one.
    Updated { entries: usize },
    /// The source answered `304 Not Modified`.
    NotModified,
}

/// Validators and timestamps of the cached copy (`meta.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_modified: Option<String>,
    /// Source the validators came from; an `ETag` means nothing to another URL.
    #[serde(default)]
    source_url: String,
    /// SHA-256 (hex) of the cached body the validators describe. The body and
    /// meta files are renamed one after the other, so a crash between the two
    /// can leave them out of step; the hash exposes that on load.
    #[serde(default)]
    body_sha256: String,
    /// When the cached body was last replaced.
    fetched_at: DateTime<Utc>,
    /// When the source was last asked, whether or not the body changed.
    checked_at: DateTime<Utc>,
}

pub struct CatalogSync {
    config: CatalogConfig,
    client: reqwest::Client,
    cache_dir: PathBuf,
    clock: Arc<dyn Clock>,
    refresh_interval: Duration,
    max_body_bytes: usize,
    index: RwLock<Arc<CatalogIndex>>,
    /// Meta of the copy behind `index`; held across a refresh so two
    /// refreshes never interleave.
    meta: Mutex<Option<CacheMeta>>,
}

impl CatalogSync {
    /// A sync whose cache lives in `<base_dir>/cache/integrations/`.
    pub fn new(config: CatalogConfig, base_dir: &Path) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("assistant-integrations-catalog")
            .build()?;
        Ok(Self {
            config,
            client,
            cache_dir: base_dir.join("cache").join("integrations"),
            clock: Arc::new(SystemClock),
            refresh_interval: REFRESH_INTERVAL,
            max_body_bytes: MAX_BODY_BYTES,
            index: RwLock::new(Arc::new(CatalogIndex::empty())),
            meta: Mutex::new(None),
        })
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn with_refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }

    pub fn with_max_body_bytes(mut self, limit: usize) -> Self {
        self.max_body_bytes = limit;
        self
    }

    /// The current catalog. Cheap: clones an `Arc`.
    pub async fn index(&self) -> Arc<CatalogIndex> {
        self.index.read().await.clone()
    }

    /// Load the cached copy from disk, if any, without touching the network.
    ///
    /// A missing or unreadable cache leaves the index as it was.
    pub async fn load_cached(&self) {
        if !self.config.enabled {
            return;
        }
        // Held until the index is published so a refresh cannot slip a newer
        // body in between the read and the meta update.
        let mut cached_meta = self.meta.lock().await;
        let bytes = match tokio::fs::read(self.cache_dir.join(CATALOG_FILE)).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                warn!("cannot read the cached integrations catalog: {e}");
                return;
            }
        };
        let index = match parse_catalog(&bytes) {
            Ok(index) => index,
            Err(e) => {
                warn!("ignoring the cached integrations catalog: {e:#}");
                return;
            }
        };
        info!(
            entries = index.len(),
            "loaded the cached integrations catalog"
        );
        *self.index.write().await = Arc::new(index);
        *cached_meta = self
            .read_meta()
            .await
            .filter(|meta| self.describes(meta, &bytes));
    }

    /// Whether `meta` holds validators for `body` as fetched from the
    /// configured source.
    fn describes(&self, meta: &CacheMeta, body: &[u8]) -> bool {
        let matches =
            meta.source_url == self.config.source_url && meta.body_sha256 == body_digest(body);
        if !matches {
            info!("cached integrations catalog validators do not match; fetching unconditionally");
        }
        matches
    }

    /// Fetch the source once, conditionally when a cached copy exists.
    ///
    /// On any failure the cached copy stays in place.
    pub async fn refresh(&self) -> Result<RefreshOutcome> {
        if !self.config.enabled {
            bail!("the integrations catalog is disabled");
        }
        let mut cached_meta = self.meta.lock().await;

        let mut request = self.client.get(&self.config.source_url);
        if let Some(meta) = cached_meta.as_ref() {
            if let Some(etag) = &meta.etag {
                request = request.header(IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = &meta.last_modified {
                request = request.header(IF_MODIFIED_SINCE, last_modified);
            }
        }
        let response = request
            .send()
            .await
            .context("integrations catalog request failed")?;
        let status = response.status();

        if status == StatusCode::NOT_MODIFIED {
            let Some(meta) = cached_meta.as_mut() else {
                bail!("catalog source answered 304 to an unconditional request");
            };
            meta.checked_at = self.clock.now();
            if let Err(e) = self.persist_meta(meta).await {
                warn!("cannot persist the integrations catalog metadata: {e:#}");
            }
            return Ok(RefreshOutcome::NotModified);
        }
        if !status.is_success() {
            bail!("catalog source answered {status}");
        }

        let etag = header_value(&response, ETAG);
        let last_modified = header_value(&response, LAST_MODIFIED);
        let bytes = read_limited(response, self.max_body_bytes).await?;
        let index = parse_catalog(&bytes)?;
        let entries = index.len();

        let now = self.clock.now();
        let mut meta = CacheMeta {
            etag,
            last_modified,
            source_url: self.config.source_url.clone(),
            body_sha256: body_digest(&bytes),
            fetched_at: now,
            checked_at: now,
        };
        if let Err(e) = self.persist(&bytes, &meta).await {
            warn!("cannot persist the integrations catalog: {e:#}");
            // The disk still holds an older body. Withhold the validators so
            // the next refresh fetches in full and saves body and meta together,
            // instead of a 304 persisting validators for a body that is not there.
            meta.etag = None;
            meta.last_modified = None;
        }
        *self.index.write().await = Arc::new(index);
        *cached_meta = Some(meta);
        Ok(RefreshOutcome::Updated { entries })
    }

    /// Load the disk cache, then refresh now and every refresh interval in a
    /// background task. Does nothing when the catalog is disabled.
    pub async fn start(self: &Arc<Self>) {
        if !self.config.enabled {
            info!("integrations catalog disabled; not fetching");
            return;
        }
        self.load_cached().await;

        let sync = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(sync.refresh_interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                match sync.refresh().await {
                    Ok(outcome) => info!(?outcome, "refreshed the integrations catalog"),
                    Err(e) => {
                        warn!("integrations catalog refresh failed, keeping the cached copy: {e:#}")
                    }
                }
            }
        });
    }

    async fn read_meta(&self) -> Option<CacheMeta> {
        let bytes = tokio::fs::read(self.cache_dir.join(META_FILE)).await.ok()?;
        match serde_json::from_slice(&bytes) {
            Ok(meta) => Some(meta),
            Err(e) => {
                warn!("ignoring unreadable integrations catalog metadata: {e}");
                None
            }
        }
    }

    async fn persist(&self, body: &[u8], meta: &CacheMeta) -> Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .context("cannot create the catalog cache directory")?;
        write_atomic(&self.cache_dir.join(CATALOG_FILE), body).await?;
        self.persist_meta(meta).await
    }

    async fn persist_meta(&self, meta: &CacheMeta) -> Result<()> {
        write_atomic(
            &self.cache_dir.join(META_FILE),
            &serde_json::to_vec_pretty(meta)?,
        )
        .await
    }
}

fn parse_catalog(bytes: &[u8]) -> Result<CatalogIndex> {
    let document: serde_json::Value =
        serde_json::from_slice(bytes).context("catalog body is not JSON")?;
    CatalogIndex::from_document(&document)
        .ok_or_else(|| anyhow::anyhow!("catalog body has no `data` array"))
}

fn body_digest(body: &[u8]) -> String {
    hex::encode(Sha256::digest(body))
}

/// Read a response body, giving up as soon as it passes `limit` bytes.
async fn read_limited(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|len| len > limit as u64)
    {
        bail!("catalog body is over the {limit} byte limit");
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("cannot read the integrations catalog body")?
    {
        if body.len() + chunk.len() > limit {
            bail!("catalog body is over the {limit} byte limit");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn header_value(response: &reqwest::Response, name: HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Write `bytes` to a sibling temp file and rename it over `path`, so a
/// reader never sees a half-written file.
async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    let result = async {
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(&tmp, path).await
    }
    .await;
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e).with_context(|| format!("cannot write {}", path.display()));
    }
    Ok(())
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::clock::FakeClock;

    const FIXTURE: &str = include_str!("../../core/tests/fixtures/integrations-catalog.json");
    /// Entries of the fixture that are installable remote MCP servers.
    const FIXTURE_ENTRIES: usize = 13;

    fn config_for(server: &MockServer) -> CatalogConfig {
        CatalogConfig {
            enabled: true,
            source_url: format!("{}/api.json", server.uri()),
        }
    }

    fn sync_in(dir: &Path, config: CatalogConfig) -> CatalogSync {
        CatalogSync::new(config, dir).unwrap()
    }

    fn cache_file(dir: &Path, name: &str) -> PathBuf {
        dir.join("cache").join("integrations").join(name)
    }

    fn read_meta(dir: &Path) -> serde_json::Value {
        let raw = std::fs::read_to_string(cache_file(dir, META_FILE)).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    fn catalog_response(etag: &str) -> ResponseTemplate {
        ResponseTemplate::new(200)
            .insert_header("etag", etag)
            .set_body_raw(FIXTURE, "application/json")
    }

    /// A body with `n` installable entries, to tell two catalogs apart.
    fn catalog_of(n: usize) -> String {
        let data: Vec<serde_json::Value> = (0..n)
            .map(|i| {
                serde_json::json!({
                    "id": format!("e/{i}"), "kind": "mcp", "name": format!("Entry {i}"),
                    "connectUrl": "https://e.example/mcp"
                })
            })
            .collect();
        serde_json::json!({"version": 1, "data": data}).to_string()
    }

    async fn mount_catalog(server: &MockServer, etag: &str) {
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(catalog_response(etag))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn first_fetch_serves_and_persists_the_catalog() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        let outcome = sync.refresh().await.unwrap();

        assert_eq!(
            outcome,
            RefreshOutcome::Updated {
                entries: FIXTURE_ENTRIES
            }
        );
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
        let on_disk = std::fs::read_to_string(cache_file(dir.path(), CATALOG_FILE)).unwrap();
        assert_eq!(on_disk, FIXTURE, "the raw upstream body is cached");
        assert_eq!(read_meta(dir.path())["etag"], "\"v1\"");
    }

    #[tokio::test]
    async fn first_request_is_unconditional() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        sync.refresh().await.unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].headers.get("if-none-match").is_none());
        assert!(requests[0].headers.get("if-modified-since").is_none());
    }

    #[tokio::test]
    async fn not_modified_keeps_the_copy_and_only_bumps_checked_at() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .and(header("if-none-match", "\"v1\""))
            .respond_with(ResponseTemplate::new(304))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(catalog_response("\"v1\""))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::new_at_seed());
        let sync = sync_in(dir.path(), config_for(&server)).with_clock(clock.clone());
        sync.refresh().await.unwrap();
        let before = read_meta(dir.path());
        clock.advance(Duration::from_secs(3600));

        let outcome = sync.refresh().await.unwrap();

        assert_eq!(outcome, RefreshOutcome::NotModified);
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
        let after = read_meta(dir.path());
        assert_eq!(after["fetched_at"], before["fetched_at"]);
        assert_ne!(after["checked_at"], before["checked_at"]);
        assert_eq!(after["etag"], before["etag"]);
    }

    #[tokio::test]
    async fn last_modified_is_replayed_as_if_modified_since() {
        let server = MockServer::start().await;
        let stamp = "Wed, 25 Nov 2026 09:00:00 GMT";
        Mock::given(method("GET"))
            .and(path("/api.json"))
            // `header()` splits on the commas an HTTP date contains.
            .and(move |request: &wiremock::Request| {
                request
                    .headers
                    .get("if-modified-since")
                    .is_some_and(|v| v == stamp)
            })
            .respond_with(ResponseTemplate::new(304))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(catalog_response("\"v1\"").insert_header("last-modified", stamp))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));
        sync.refresh().await.unwrap();

        let outcome = sync.refresh().await.unwrap();

        assert_eq!(outcome, RefreshOutcome::NotModified);
    }

    #[tokio::test]
    async fn changed_upstream_replaces_the_copy() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .and(header("if-none-match", "\"v1\""))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "\"v2\"")
                    .set_body_raw(catalog_of(3), "application/json"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(catalog_response("\"v1\""))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));
        sync.refresh().await.unwrap();

        let outcome = sync.refresh().await.unwrap();

        assert_eq!(outcome, RefreshOutcome::Updated { entries: 3 });
        assert_eq!(sync.index().await.len(), 3);
        assert_eq!(read_meta(dir.path())["etag"], "\"v2\"");
    }

    #[tokio::test]
    async fn garbage_200_keeps_the_cached_copy() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw("<html>maintenance</html>", "text/html"),
            )
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));
        seed_cache(dir.path(), &sync).await;

        let result = sync.refresh().await;

        assert!(result.is_err());
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
        let on_disk = std::fs::read_to_string(cache_file(dir.path(), CATALOG_FILE)).unwrap();
        assert_eq!(on_disk, FIXTURE);
    }

    #[tokio::test]
    async fn json_that_is_not_a_catalog_keeps_the_cached_copy() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));
        seed_cache(dir.path(), &sync).await;

        assert!(sync.refresh().await.is_err());

        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }

    #[tokio::test]
    async fn server_error_keeps_the_cached_copy() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));
        seed_cache(dir.path(), &sync).await;

        assert!(sync.refresh().await.is_err());

        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }

    #[tokio::test]
    async fn network_error_keeps_the_cached_copy() {
        let dir = tempfile::tempdir().unwrap();
        let unreachable = {
            // Bind to learn a free port, then close it again.
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://{}/api.json", listener.local_addr().unwrap())
        };
        let sync = sync_in(
            dir.path(),
            CatalogConfig {
                enabled: true,
                source_url: unreachable,
            },
        );
        seed_cache(dir.path(), &sync).await;

        assert!(sync.refresh().await.is_err());

        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }

    #[tokio::test]
    async fn restart_serves_the_disk_copy_before_any_fetch() {
        let first = MockServer::start().await;
        mount_catalog(&first, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        sync_in(dir.path(), config_for(&first))
            .refresh()
            .await
            .unwrap();

        let second = MockServer::start().await;
        let restarted = sync_in(dir.path(), config_for(&second));
        restarted.load_cached().await;

        assert_eq!(restarted.index().await.len(), FIXTURE_ENTRIES);
        assert!(second.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn restart_reuses_the_cached_validators() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .and(header("if-none-match", "\"v1\""))
            .respond_with(ResponseTemplate::new(304))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api.json"))
            .respond_with(catalog_response("\"v1\""))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        sync_in(dir.path(), config_for(&server))
            .refresh()
            .await
            .unwrap();
        let restarted = sync_in(dir.path(), config_for(&server));
        restarted.load_cached().await;

        let outcome = restarted.refresh().await.unwrap();

        assert_eq!(outcome, RefreshOutcome::NotModified);
    }

    #[tokio::test]
    async fn corrupt_disk_cache_is_ignored() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(cache_file(dir.path(), CATALOG_FILE).parent().unwrap()).unwrap();
        std::fs::write(cache_file(dir.path(), CATALOG_FILE), "{ not json").unwrap();
        std::fs::write(
            cache_file(dir.path(), META_FILE),
            r#"{"etag":"\"v1\"","fetched_at":"2026-11-25T00:00:00Z","checked_at":"2026-11-25T00:00:00Z"}"#,
        )
        .unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        sync.load_cached().await;

        assert!(sync.index().await.is_empty());
    }

    #[tokio::test]
    async fn disabled_catalog_makes_no_requests_and_stays_empty() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        // A previous run left a perfectly good cache behind.
        let seeded = sync_in(dir.path(), config_for(&server));
        seeded.refresh().await.unwrap();
        server.reset().await;
        let sync = Arc::new(
            sync_in(
                dir.path(),
                CatalogConfig {
                    enabled: false,
                    ..config_for(&server)
                },
            )
            .with_refresh_interval(Duration::from_millis(10)),
        );

        sync.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(server.received_requests().await.unwrap().is_empty());
        assert!(sync.index().await.is_empty());
    }

    #[tokio::test]
    async fn start_fetches_at_once_and_then_on_every_interval() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        let sync = Arc::new(
            sync_in(dir.path(), config_for(&server))
                .with_refresh_interval(Duration::from_millis(50)),
        );

        sync.start().await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while server.received_requests().await.unwrap().len() < 3 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "expected an initial fetch and two refreshes"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }

    #[tokio::test]
    async fn no_temp_files_are_left_behind() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        sync.refresh().await.unwrap();

        let mut names: Vec<String> =
            std::fs::read_dir(cache_file(dir.path(), CATALOG_FILE).parent().unwrap())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
        names.sort();
        assert_eq!(names, [CATALOG_FILE, META_FILE]);
    }

    // -- Review follow-ups ---------------------------------------------------

    fn unconditional(request: &wiremock::Request) -> bool {
        request.headers.get("if-none-match").is_none()
            && request.headers.get("if-modified-since").is_none()
    }

    #[tokio::test]
    async fn load_cached_holds_the_meta_lock_until_it_has_published_the_index() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let sync = Arc::new(sync_in(dir.path(), config_for(&server)));
        seed_cache(dir.path(), &sync_in(dir.path(), config_for(&server))).await;
        let guard = sync.meta.lock().await;

        let loader = {
            let sync = sync.clone();
            tokio::spawn(async move { sync.load_cached().await })
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        let loaded_while_locked = !sync.index().await.is_empty();
        drop(guard);
        loader.await.unwrap();

        assert!(
            !loaded_while_locked,
            "load_cached must wait for the meta lock"
        );
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }

    #[tokio::test]
    async fn validators_cached_for_another_source_are_not_replayed() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a.json"))
            .respond_with(catalog_response("\"v1\""))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/b.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "\"v1\"")
                    .set_body_raw(catalog_of(3), "application/json"),
            )
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let from_a = CatalogConfig {
            enabled: true,
            source_url: format!("{}/a.json", server.uri()),
        };
        let from_b = CatalogConfig {
            enabled: true,
            source_url: format!("{}/b.json", server.uri()),
        };
        sync_in(dir.path(), from_a).refresh().await.unwrap();
        let mirror = sync_in(dir.path(), from_b);
        mirror.load_cached().await;

        let outcome = mirror.refresh().await.unwrap();

        assert_eq!(outcome, RefreshOutcome::Updated { entries: 3 });
        let requests = server.received_requests().await.unwrap();
        assert!(
            unconditional(&requests[1]),
            "mirror request: {:?}",
            requests[1].headers
        );
    }

    #[tokio::test]
    async fn validators_are_withheld_while_the_body_is_not_on_disk() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        // A file where the cache directory should be makes every write fail.
        std::fs::create_dir_all(dir.path().join("cache")).unwrap();
        std::fs::write(dir.path().join("cache").join("integrations"), "in the way").unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        sync.refresh().await.unwrap();
        assert_eq!(
            sync.index().await.len(),
            FIXTURE_ENTRIES,
            "still served from memory"
        );
        sync.refresh().await.unwrap();

        let requests = server.received_requests().await.unwrap();
        assert!(
            unconditional(&requests[1]),
            "second request: {:?}",
            requests[1].headers
        );
    }

    #[tokio::test]
    async fn validators_that_do_not_describe_the_cached_body_are_dropped_on_load() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v2\"").await;
        let dir = tempfile::tempdir().unwrap();
        let cache = cache_file(dir.path(), CATALOG_FILE);
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        // Body B on disk, validators of body A: a crash between the two writes.
        std::fs::write(&cache, catalog_of(3)).unwrap();
        std::fs::write(
            cache_file(dir.path(), META_FILE),
            serde_json::json!({
                "etag": "\"v1\"",
                "source_url": config_for(&server).source_url,
                "body_sha256": "0000",
                "fetched_at": "2026-11-25T00:00:00Z",
                "checked_at": "2026-11-25T00:00:00Z"
            })
            .to_string(),
        )
        .unwrap();
        let sync = sync_in(dir.path(), config_for(&server));

        sync.load_cached().await;
        assert_eq!(sync.index().await.len(), 3, "the body is still served");
        sync.refresh().await.unwrap();

        let requests = server.received_requests().await.unwrap();
        assert!(
            unconditional(&requests[0]),
            "request: {:?}",
            requests[0].headers
        );
    }

    #[tokio::test]
    async fn a_body_over_the_size_limit_is_rejected_and_the_cache_kept() {
        let server = MockServer::start().await;
        mount_catalog(&server, "\"v1\"").await;
        let dir = tempfile::tempdir().unwrap();
        let sync = sync_in(dir.path(), config_for(&server)).with_max_body_bytes(1024);

        let result = sync.refresh().await;

        assert!(result.is_err());
        assert!(sync.index().await.is_empty());
        assert!(!cache_file(dir.path(), CATALOG_FILE).exists());
    }

    /// Put a good copy in `sync`'s memory and on disk, as an earlier run would.
    async fn seed_cache(dir: &Path, sync: &CatalogSync) {
        let good = MockServer::start().await;
        mount_catalog(&good, "\"v1\"").await;
        sync_in(dir, config_for(&good)).refresh().await.unwrap();
        sync.load_cached().await;
        assert_eq!(sync.index().await.len(), FIXTURE_ENTRIES);
    }
}
