//! Skills that external MCP servers publish: discovery, verification against
//! the manifest and the cache that keeps verified files.

mod skills_server;

use std::path::Path;

use assistant_mcp_client::McpSkillSource;
use assistant_skills::{SkillDef, SkillSource};
use rmcp::model::{SkillResource, SkillResources};
use skills_server::{SkillServer, skill_md, uri};

const SCRIPT: &str = "#!/bin/sh\necho deployed\n";

fn cache(base: &Path) -> std::path::PathBuf {
    base.join("cache").join("mcp-skills")
}

async fn offered(server: &SkillServer, base: &Path) -> Vec<SkillDef> {
    let client = server.connect("acme").await;
    McpSkillSource::new(base).skills(&client).await
}

#[tokio::test]
async fn a_published_skill_is_offered_as_server_prefixed_name_with_its_files_cached() {
    let server = SkillServer::new();
    server.publish(
        "deploy",
        "Ship it",
        "Run the script.",
        &[("scripts/run.sh", SCRIPT)],
    );
    let base = tempfile::tempdir().unwrap();

    let skills = offered(&server, base.path()).await;

    assert_eq!(skills.len(), 1, "one skill is published");
    let skill = &skills[0];
    assert_eq!(skill.name, "acme__deploy");
    assert_eq!(skill.description, "Ship it");
    assert_eq!(skill.body.trim(), "Run the script.");
    assert_eq!(
        skill.source,
        SkillSource::Mcp {
            server: "acme".into()
        }
    );
    assert_eq!(skill.dir, cache(base.path()).join("acme").join("deploy"));
    assert_eq!(
        std::fs::read_to_string(skill.dir.join("scripts/run.sh")).unwrap(),
        SCRIPT,
        "bundled files are written to the cache"
    );
}

#[tokio::test]
async fn a_server_without_the_extension_is_not_asked_for_skills() {
    let server = SkillServer::without_the_extension();
    let base = tempfile::tempdir().unwrap();

    let skills = offered(&server, base.path()).await;

    assert!(skills.is_empty(), "nothing to offer");
    assert_eq!(server.list_calls(), 0, "skills/list is never sent");
}

#[tokio::test]
async fn a_file_with_the_wrong_digest_rejects_the_skill() {
    let server = SkillServer::new();
    let manifest = vec![
        SkillResource::from_bytes(
            uri("deploy", "SKILL.md"),
            skill_md("deploy", "Ship it", "body").as_bytes(),
        ),
        SkillResource::from_bytes(uri("deploy", "scripts/run.sh"), b"what the manifest says"),
    ];
    server.publish_with_manifest(
        "deploy",
        "Ship it",
        SkillResources::Files(manifest),
        vec![
            ("SKILL.md".into(), skill_md("deploy", "Ship it", "body")),
            ("scripts/run.sh".into(), SCRIPT.into()),
        ],
    );
    let base = tempfile::tempdir().unwrap();

    let skills = offered(&server, base.path()).await;

    assert!(skills.is_empty(), "a tampered skill is not offered");
    assert!(
        !cache(base.path()).join("acme/deploy").exists(),
        "its files are not left in the cache"
    );
}

#[tokio::test]
async fn a_file_with_the_wrong_size_rejects_the_skill() {
    let server = SkillServer::new();
    let md = skill_md("deploy", "Ship it", "body");
    let mut resource = SkillResource::from_bytes(uri("deploy", "SKILL.md"), md.as_bytes());
    resource.size += 1;
    server.publish_with_manifest(
        "deploy",
        "Ship it",
        SkillResources::Files(vec![resource]),
        vec![("SKILL.md".into(), md)],
    );
    let base = tempfile::tempdir().unwrap();

    assert!(offered(&server, base.path()).await.is_empty());
}

#[tokio::test]
async fn frontmatter_that_disagrees_with_the_manifest_rejects_the_skill() {
    let server = SkillServer::new();
    let md = skill_md("deploy", "What the file says", "body");
    server.publish_with_manifest(
        "deploy",
        "What the manifest says",
        SkillResources::Files(vec![SkillResource::from_bytes(
            uri("deploy", "SKILL.md"),
            md.as_bytes(),
        )]),
        vec![("SKILL.md".into(), md)],
    );
    let base = tempfile::tempdir().unwrap();

    assert!(offered(&server, base.path()).await.is_empty());
}

#[tokio::test]
async fn a_dynamic_skill_is_fetched_without_a_digest_check() {
    let server = SkillServer::new();
    server.publish_with_manifest(
        "deploy",
        "Ship it",
        SkillResources::Dynamic,
        vec![(
            "SKILL.md".into(),
            skill_md("deploy", "Ship it", "dynamic body"),
        )],
    );
    let base = tempfile::tempdir().unwrap();

    let skills = offered(&server, base.path()).await;

    assert_eq!(skills.len(), 1, "a dynamic skill is offered");
    assert_eq!(skills[0].body.trim(), "dynamic body");
    assert_eq!(server.reads(), vec![uri("deploy", "SKILL.md")]);
}

#[tokio::test]
async fn an_unchanged_manifest_is_served_from_the_cache() {
    let server = SkillServer::new();
    server.publish("deploy", "Ship it", "body", &[("scripts/run.sh", SCRIPT)]);
    let base = tempfile::tempdir().unwrap();
    offered(&server, base.path()).await;
    let reads = server.reads().len();

    let skills = offered(&server, base.path()).await;

    assert_eq!(skills.len(), 1, "still offered");
    assert_eq!(server.reads().len(), reads, "no file is fetched again");
}

#[tokio::test]
async fn a_changed_manifest_forces_the_skill_to_be_verified_again() {
    let server = SkillServer::new();
    server.publish("deploy", "Ship it", "first body", &[]);
    let base = tempfile::tempdir().unwrap();
    offered(&server, base.path()).await;

    // The server swaps in a new SKILL.md but keeps the old digest.
    let md = skill_md("deploy", "Ship it", "second body");
    let stale = SkillResource::from_bytes(
        uri("deploy", "SKILL.md"),
        skill_md("deploy", "Ship it", "first body").as_bytes(),
    );
    let changed = SkillResource::new(
        stale.uri.clone(),
        "sha256:".to_string() + &"0".repeat(64),
        stale.size,
    );
    server.publish_with_manifest(
        "deploy",
        "Ship it",
        SkillResources::Files(vec![changed]),
        vec![("SKILL.md".into(), md)],
    );

    let skills = offered(&server, base.path()).await;

    assert!(
        skills.is_empty(),
        "the cached copy does not stand in for a manifest that no longer verifies"
    );

    // A manifest that does verify is picked up and replaces the cache.
    server.publish("deploy", "Ship it", "second body", &[]);
    let skills = offered(&server, base.path()).await;
    assert_eq!(skills[0].body.trim(), "second body");
}

#[tokio::test]
async fn a_cached_file_that_was_altered_on_disk_is_fetched_again() {
    let server = SkillServer::new();
    server.publish("deploy", "Ship it", "body", &[("scripts/run.sh", SCRIPT)]);
    let base = tempfile::tempdir().unwrap();
    let skills = offered(&server, base.path()).await;
    std::fs::write(skills[0].dir.join("scripts/run.sh"), "rm -rf /").unwrap();

    let skills = offered(&server, base.path()).await;

    assert_eq!(
        std::fs::read_to_string(skills[0].dir.join("scripts/run.sh")).unwrap(),
        SCRIPT,
        "the file is restored from the server"
    );
}

#[tokio::test]
async fn sync_publishes_each_servers_skills_and_empties_a_disconnected_one() {
    use std::sync::{Arc, Mutex};

    use assistant_mcp_client::skills::SkillPublisher;

    let server = SkillServer::new();
    server.publish("deploy", "Ship it", "Run it.", &[]);
    let client = Arc::new(server.connect("acme").await);
    let base = tempfile::tempdir().unwrap();
    let published: Arc<Mutex<Vec<(String, Vec<String>)>>> = Arc::default();
    let sink = published.clone();
    let publish: SkillPublisher = Arc::new(move |server, skills| {
        let sink = sink.clone();
        Box::pin(async move {
            let names = skills.into_iter().map(|s| s.name).collect();
            sink.lock().unwrap().push((server, names));
        })
    });

    McpSkillSource::new(base.path())
        .sync(
            &[("acme".into(), Some(client)), ("gone".into(), None)],
            &publish,
        )
        .await;

    assert_eq!(
        *published.lock().unwrap(),
        vec![
            ("acme".to_string(), vec!["acme__deploy".to_string()]),
            ("gone".to_string(), Vec::new()),
        ]
    );
}
