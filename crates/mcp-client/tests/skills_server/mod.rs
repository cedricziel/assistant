//! An in-process rmcp server that publishes skills through the Skills
//! extension, for the remote-skill tests.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use assistant_mcp_client::StaleTools;
use assistant_mcp_client::client::McpClient;
use rmcp::model::{
    ListSkillsResult, PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, ResourceContents, ServerCapabilities, ServerConfig, Skill,
    SkillFrontmatter, SkillResource, SkillResources,
};
use rmcp::service::{MaybeSendFuture, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};

pub fn skill_md(name: &str, description: &str, body: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n")
}

pub fn uri(skill: &str, path: &str) -> String {
    format!("skill://acme/{skill}/{path}")
}

#[derive(Clone)]
pub struct SkillServer {
    advertises_skills: bool,
    skills: Arc<Mutex<Vec<Skill>>>,
    files: Arc<Mutex<HashMap<String, String>>>,
    pub list_calls: Arc<AtomicUsize>,
    pub reads: Arc<Mutex<Vec<String>>>,
}

impl SkillServer {
    pub fn new() -> Self {
        Self {
            advertises_skills: true,
            skills: Arc::default(),
            files: Arc::default(),
            list_calls: Arc::default(),
            reads: Arc::default(),
        }
    }

    pub fn without_the_extension() -> Self {
        Self {
            advertises_skills: false,
            ..Self::new()
        }
    }

    pub fn list_calls(&self) -> usize {
        self.list_calls.load(Ordering::SeqCst)
    }

    pub fn reads(&self) -> Vec<String> {
        self.reads.lock().unwrap().clone()
    }

    /// Publish `name` with a correct manifest over `files` (path, content).
    /// `SKILL.md` is `skill_md(name, description, body)`.
    pub fn publish(&self, name: &str, description: &str, body: &str, extra: &[(&str, &str)]) {
        let mut files = vec![("SKILL.md".to_string(), skill_md(name, description, body))];
        files.extend(extra.iter().map(|(p, c)| (p.to_string(), c.to_string())));
        let manifest = files
            .iter()
            .map(|(path, content)| SkillResource::from_bytes(uri(name, path), content.as_bytes()))
            .collect();
        self.publish_with_manifest(name, description, SkillResources::Files(manifest), files);
    }

    /// Publish `name` with an arbitrary manifest, over files that may not
    /// match it.
    pub fn publish_with_manifest(
        &self,
        name: &str,
        description: &str,
        resources: SkillResources,
        files: Vec<(String, String)>,
    ) {
        let skill = Skill::new(
            uri(name, "SKILL.md"),
            SkillFrontmatter::new(name, description),
            resources,
        );
        let mut skills = self.skills.lock().unwrap();
        skills.retain(|s| s.name() != name);
        skills.push(skill);
        let mut stored = self.files.lock().unwrap();
        for (path, content) in files {
            stored.insert(uri(name, &path), content);
        }
    }

    pub async fn connect(&self, name: &str) -> McpClient {
        let (server_io, client_io) = tokio::io::duplex(65536);
        let server = self.clone();
        tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        McpClient::connect(name, client_io, StaleTools::default())
            .await
            .expect("the client connects")
    }
}

impl ServerHandler for SkillServer {
    fn get_info(&self) -> ServerConfig {
        let capabilities = if self.advertises_skills {
            ServerCapabilities::builder()
                .enable_resources()
                .enable_skills()
                .build()
        } else {
            ServerCapabilities::builder().enable_resources().build()
        };
        ServerConfig::new(capabilities)
    }

    fn list_skills(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListSkillsResult, ErrorData>> + MaybeSendFuture + '_ {
        self.list_calls.fetch_add(1, Ordering::SeqCst);
        let skills = self.skills.lock().unwrap().clone();
        std::future::ready(Ok(ListSkillsResult::with_all_items(skills)))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResponse, ErrorData>> + MaybeSendFuture + '_ {
        self.reads.lock().unwrap().push(request.uri.clone());
        let content = self.files.lock().unwrap().get(&request.uri).cloned();
        std::future::ready(match content {
            Some(text) => Ok(ReadResourceResponse::Complete(ReadResourceResult::new(
                vec![ResourceContents::text(text, request.uri)],
            ))),
            None => Err(ErrorData::resource_not_found("no such file", None)),
        })
    }
}
