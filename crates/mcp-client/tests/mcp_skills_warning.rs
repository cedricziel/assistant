//! A skill that fails verification is reported in a warning that names the
//! server and the skill.
//!
//! Alone in its test binary: `tracing` caches which callsites are enabled, so
//! a capture shared with parallel tests can miss events.

mod skills_server;

use std::sync::{Arc, Mutex};

use assistant_mcp_client::McpSkillSource;
use rmcp::model::{SkillResource, SkillResources};
use skills_server::{SkillServer, skill_md, uri};

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
async fn a_digest_mismatch_warns_with_the_server_and_the_skill() {
    let logs = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let server = SkillServer::new();
    let md = skill_md("deploy", "Ship it", "body");
    let wrong = SkillResource::new(
        uri("deploy", "SKILL.md"),
        format!("sha256:{}", "0".repeat(64)),
        md.len() as u64,
    );
    server.publish_with_manifest(
        "deploy",
        "Ship it",
        SkillResources::Files(vec![wrong]),
        vec![("SKILL.md".into(), md)],
    );
    let base = tempfile::tempdir().unwrap();
    let client = server.connect("acme").await;

    let skills = McpSkillSource::new(base.path()).skills(&client).await;

    assert!(skills.is_empty(), "the skill is not offered");
    let log = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    let warning = log
        .lines()
        .find(|line| line.contains("WARN") && line.contains("remote skill rejected"))
        .unwrap_or_else(|| panic!("a warning is logged, got: {log}"));
    assert!(
        warning.contains("acme") && warning.contains("deploy") && warning.contains("digest"),
        "the warning names the server, the skill and the reason, got: {warning}"
    );
}
