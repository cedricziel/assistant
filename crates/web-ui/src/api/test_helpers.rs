//! Shared test fixtures for `api::*` handler tests.
//!
//! Each sibling handler module declares its own `#[cfg(test)] mod tests`;
//! all of them pull fixtures from here via
//! `use super::super::test_helpers::*;`.

#![cfg(test)]

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use http_body_util::BodyExt;
use tokio::sync::RwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::types::agent::OrgConfig;
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_llm_provider::retry::RetryConfig;
use assistant_runtime::{OrgContexts, SpaceRuntimes};
use assistant_storage::StorageLayer;
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};

use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{OrgStore as _, Organization, Space, SpaceStore as _};
use assistant_storage::{OrgPoolFactory, OrgStorageLayer};

use crate::space::SpaceResolver;

use super::{ApiState, api_router};

/// One seeded space for [`resolver_for`].
pub(crate) struct TestSpace {
    pub id: &'static str,
    pub org_id: &'static str,
    pub org_slug: &'static str,
    pub slug: &'static str,
    pub storage: Arc<StorageLayer>,
}

/// Build a [`SpaceResolver`] over an in-memory org database that holds
/// `spaces`, each served from its preloaded storage (nothing touches disk).
pub(crate) async fn resolver_for(spaces: &[TestSpace]) -> SpaceResolver {
    resolver_in(Path::new("/nonexistent/assistant-test-root"), spaces).await
}

/// Like [`resolver_for`], with the install root at `root`: skills and other
/// files that runtimes write land there.
pub(crate) async fn resolver_in(root: &Path, spaces: &[TestSpace]) -> SpaceResolver {
    let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
    let pools = Arc::new(OrgPoolFactory::new(root));
    let now = chrono::DateTime::UNIX_EPOCH;
    let mut orgs_seen = std::collections::HashSet::new();
    for space in spaces {
        if orgs_seen.insert(space.org_id) {
            org_storage
                .org_store()
                .create_org(&Organization {
                    id: OrgId::from(space.org_id),
                    name: space.org_slug.into(),
                    slug: space.org_slug.into(),
                    auth_mode: "password".into(),
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
        }
        org_storage
            .space_store()
            .create_space(&Space {
                id: SpaceId::from(space.id),
                org_id: OrgId::from(space.org_id),
                name: space.slug.into(),
                slug: space.slug.into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        pools
            .insert_space_storage(space.org_slug, space.slug, space.storage.clone())
            .unwrap();
    }
    SpaceResolver::new(org_storage, pools)
}

/// Resolver with the single `default`/`default` space that
/// [`AuthContext::system`](assistant_core::auth::AuthContext::system) holds a
/// role in, served from `storage`.
pub(crate) async fn default_resolver(storage: &Arc<StorageLayer>) -> SpaceResolver {
    resolver_for(&[TestSpace {
        id: "default",
        org_id: "default",
        org_slug: "default",
        slug: "default",
        storage: storage.clone(),
    }])
    .await
}

/// Upper bound for `submit_turn` in tests.
///
/// The production default is 3 hours (`submit_timeout_secs: 10_800`), which is
/// right for real agent turns but catastrophic in CI: a turn that the spawned
/// worker never picks up keeps the whole test binary blocked for three hours
/// before the handler gives up. Every fixture here talks to a mock LLM and
/// completes in milliseconds, so anything past this bound is a hung worker —
/// fail fast and surface it.
pub(crate) const TEST_SUBMIT_TIMEOUT_SECS: u64 = 30;

// -- Stubs ---------------------------------------------------------------------

/// Stub transcription provider that returns a fixed transcript.
pub(crate) struct StubTranscriptionProvider {
    pub(crate) transcript: String,
}

#[async_trait]
impl TranscriptionProvider for StubTranscriptionProvider {
    fn name(&self) -> &str {
        "stub"
    }

    async fn transcribe(
        &self,
        _request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        Ok(TranscriptionResult {
            text: self.transcript.clone(),
            language: None,
            duration_secs: None,
        })
    }
}

// -- HTTP / LLM fixtures -------------------------------------------------------

/// Minimal LLM mock: returns a static assistant reply.
pub(crate) async fn mount_llm_reply(server: &MockServer, reply: &str) {
    let body = serde_json::json!({
        "model": "test",
        "message": { "role": "assistant", "content": reply },
        "done": true
    });
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// Mount ordered Ollama-shaped replies on `/api/chat`: the first request gets
/// `replies[0]`, the second `replies[1]`, and so on. A request beyond the last
/// reply gets wiremock's default `404`.
pub(crate) async fn mount_llm_replies(server: &MockServer, replies: &[&str]) {
    for reply in replies {
        let body = serde_json::json!({
            "model": "test",
            "message": { "role": "assistant", "content": reply },
            "done": true
        });
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .up_to_n_times(1)
            .mount(server)
            .await;
    }
}

/// An `ApiState` whose runtimes are built over `spaces`' preloaded storage,
/// answering from the Ollama-shaped mock at `llm_url`. With `workers`, every
/// space's orchestrator also runs a turn worker.
pub(crate) async fn state_over(
    llm_url: &str,
    spaces: &[TestSpace],
    workers: bool,
) -> (ApiState, SpaceResolver) {
    state_over_in(
        Path::new("/nonexistent/assistant-test-root"),
        llm_url,
        spaces,
        workers,
    )
    .await
}

/// [`state_over`] with the install root at `root`.
pub(crate) async fn state_over_in(
    root: &Path,
    llm_url: &str,
    spaces: &[TestSpace],
    workers: bool,
) -> (ApiState, SpaceResolver) {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    // Follow-up suggestions make a second `/api/chat` call after every web
    // turn; tests that exercise them opt in through `state_over_config`.
    config.suggestions.enabled = false;
    state_over_config(root, llm_url, spaces, workers, config).await
}

/// [`state_over_in`] with an explicit org `config`.
pub(crate) async fn state_over_config(
    root: &Path,
    llm_url: &str,
    spaces: &[TestSpace],
    workers: bool,
    config: OrgConfig,
) -> (ApiState, SpaceResolver) {
    let llm = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: llm_url.to_string(),
            timeout_secs: 5,
            retry_config: RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let resolver = resolver_in(root, spaces).await;
    // Cap the submit timeout: the production default is 3 hours, so a turn
    // that never gets picked up by the worker would keep a test — and the CI
    // job around it — blocked for that long instead of failing fast.
    let runtimes = Arc::new(
        SpaceRuntimes::new(
            resolver.pools().clone(),
            Arc::new(OrgContexts::fixed(config, llm)),
            None,
        )
        .with_submit_timeout(TEST_SUBMIT_TIMEOUT_SECS),
    );
    if workers {
        for space in spaces {
            let runtime = runtimes.get(space.org_slug, space.slug).await.unwrap();
            let worker = runtime.orchestrator.clone();
            tokio::spawn(async move { worker.run_worker("test-worker").await });
        }
    }
    let state = ApiState::new(
        resolver.clone(),
        runtimes,
        Arc::new(RwLock::new("default".to_string())),
    );
    (state, resolver)
}

pub(crate) fn default_space(storage: &Arc<StorageLayer>) -> TestSpace {
    TestSpace {
        id: "default",
        org_id: "default",
        org_slug: "default",
        slug: "default",
        storage: storage.clone(),
    }
}

/// Build an `ApiState` wired to an in-memory DB and a mock LLM server.
pub(crate) async fn test_state(llm_url: &str) -> (ApiState, Arc<StorageLayer>) {
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let (state, _) = state_over(llm_url, &[default_space(&storage)], true).await;
    (state, storage)
}

/// Create a minimal state with a real event store but no turn worker, so the
/// single in-memory connection is not contended. Use this for tests that only
/// exercise the event log endpoints.
pub(crate) async fn event_log_state() -> (ApiState, Arc<StorageLayer>) {
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let (state, _) = state_over("http://127.0.0.1:1", &[default_space(&storage)], false).await;
    (state, storage)
}

/// Create a conversation owned by [`AuthContext::system`](assistant_core::auth::AuthContext::system),
/// the caller that [`app`] injects, and return its id.
pub(crate) async fn seed_conversation(storage: &StorageLayer) -> uuid::Uuid {
    use assistant_storage::ConversationStore as _;
    assistant_storage::SqliteConversationStore::for_agent(storage.pool.clone(), "default")
        .with_user_id("system")
        .create_conversation(None)
        .await
        .unwrap()
        .id
}

pub(crate) fn app(state: ApiState) -> axum::Router {
    // Inject a trusted system AuthContext so handlers behind the auth seam
    // (e.g. the streaming message handlers) receive an `Extension<AuthContext>`.
    app_with_auth(state, assistant_core::auth::AuthContext::system())
}

/// Like [`app`] but with a caller-supplied [`AuthContext`], so tests can
/// exercise the authorization gate (e.g. a context lacking `conversations:write`).
pub(crate) fn app_with_auth(
    state: ApiState,
    auth: assistant_core::auth::AuthContext,
) -> axum::Router {
    api_router().layer(axum::Extension(auth)).with_state(state)
}

pub(crate) async fn body_bytes(body: Body) -> Vec<u8> {
    body.collect().await.unwrap().to_bytes().to_vec()
}

pub(crate) async fn body_json(body: Body) -> serde_json::Value {
    let b = body_bytes(body).await;
    serde_json::from_slice(&b).unwrap()
}

// -- Attachment fixtures -------------------------------------------------------

pub(crate) fn tiny_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

/// Build a multipart body with a single `file` field.
pub(crate) fn multipart_body(
    field_name: &str,
    filename: &str,
    mime: &str,
    data: &[u8],
) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary123";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {mime}\r\n\r\n").as_bytes());
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}
