//! Failure logs carry the kind of a failure, never what a server echoed back.
//!
//! Alone in its test binary: `tracing` caches which callsites are enabled, so
//! a capture shared with parallel tests can miss events.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{TimeZone, Utc};
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use assistant_core::clock::FakeClock;
use assistant_core::identity::OrgId;
use assistant_core::integrations::{
    CredentialKind, CredentialMode, CredentialStatus, EffectiveIntegration,
    InMemoryIntegrationStore, Integration, IntegrationAuthKind, IntegrationCredentialStore,
    IntegrationLayer, IntegrationStore, IntegrationTrust, SecretBox, StoredCredential,
    credential_aad,
};
use assistant_core::secret::Secret;
use assistant_mcp_client::{ConnectionsConfig, IntegrationConnections};

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
async fn logs_never_contain_what_the_server_echoed_back() {
    // A server that answers `initialize` with a JSON-RPC error quoting the
    // credential it was sent, as a careless upstream or proxy might.
    let echo = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
            ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": body["id"],
                "error": {"code": -32000, "message": "rejected token ghp_ECHOED_SECRET"},
            }))
        })
        .mount(&echo)
        .await;

    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let store = Arc::new(InMemoryIntegrationStore::new());
    let integration = Integration {
        id: "int_1".into(),
        org_id: OrgId::from("org_1"),
        layer: IntegrationLayer::Org,
        space_id: None,
        owner_user_id: None,
        catalog_entry_id: None,
        connect_url: echo.uri(),
        slug: "linear".into(),
        name: "Linear".into(),
        auth_kind: IntegrationAuthKind::Header,
        header_name: Some("Authorization".into()),
        header_value_prefix: Some("Bearer ".into()),
        credential_mode: CredentialMode::Shared,
        trust: IntegrationTrust::Confirm,
        enabled: true,
        oauth_client_id_override: None,
        created_at: now,
        updated_at: now,
    };
    store.create(&integration).await.unwrap();
    let secrets = Arc::new(SecretBox::from_key([9; 32]));
    let aad = credential_aad("int_1", None, CredentialKind::Header.as_str());
    let sealed = secrets
        .seal(&Secret::new("ghp_ECHOED_SECRET"), aad.as_bytes())
        .unwrap();
    store
        .credential_store()
        .put(&StoredCredential {
            integration_id: "int_1".into(),
            subject_user_id: None,
            kind: CredentialKind::Header,
            ciphertext: sealed.ciphertext,
            nonce: sealed.nonce,
            key_id: sealed.key_id,
            expires_at: None,
            status: CredentialStatus::Ready,
            updated_at: now,
        })
        .await
        .unwrap();
    let clock = FakeClock::new(now);
    let pool = IntegrationConnections::new(
        store.clone(),
        store.credential_store(),
        store.health_store(),
        secrets,
        Arc::new(clock.clone()),
    )
    .with_config(ConnectionsConfig {
        allow_plain_http: true,
        ..ConnectionsConfig::default()
    });
    let eff = EffectiveIntegration {
        integration,
        credential_subject: None,
    };

    let logs = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        // Only this crate's own logging is under test; dependencies log theirs.
        .with_env_filter("assistant_mcp_client=trace")
        .with_ansi(false)
        .with_writer(logs.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let connect = pool.connect(&eff).await.err().unwrap();
    clock.advance(Duration::from_secs(10));
    let probe = pool.probe(&eff).await.err().unwrap();

    let logged = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("integration connection failed"),
        "the failure must still be logged: {logged}"
    );
    assert!(
        !logged.contains("ghp_ECHOED_SECRET"),
        "an echoed credential leaked into the logs: {logged}"
    );
    for error in [connect, probe] {
        assert!(!error.message().contains("ghp_ECHOED_SECRET"));
    }
}
