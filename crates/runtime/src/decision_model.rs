//! Jev decision-model client for the tool-permission pipeline.
//!
//! The model sees the tool call (rendered as one command string), the working
//! directory, the user's task and the agent name. It never sees tool output or
//! fetched content, so injected text in those cannot argue for its own
//! approval. A call is approved only when every asked question scores at least
//! the configured threshold; every other outcome yields `Ask`, never `Allow`.

use std::net::IpAddr;
use std::time::Duration;

use assistant_core::permission::{Decision, PermissionRequest, ToolDecider, Verdict};
use assistant_core::types::llm::{LlmConfig, resolve_openrouter_key};
use assistant_core::types::permissions::DecisionModelConfig;
use assistant_core::{default_workspace_dir, runtime_workspace_dir};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

const DECIDER: &str = "decision-model";
const MAX_COMMAND_CHARS: usize = 2_000;
const MAX_TASK_CHARS: usize = 2_000;
const DNS_TIMEOUT: Duration = Duration::from_secs(2);

const REVERSIBLE_INSTRUCTIONS: &str = "Running `commands` only reads or changes files inside \
`project`, or makes read-only requests, and any change can be undone with git or by running it \
again. It does not push, publish, deploy, delete files outside the project, change system \
settings, or send project data to a network service.";
const SERVES_TASK_INSTRUCTIONS: &str =
    "Running `commands` is a reasonable next step toward `task`.";

/// Asks a Jev model whether an undecided tool call is a reasonable, reversible
/// step toward the user's task.
pub struct JevDecider {
    cfg: DecisionModelConfig,
    api_key: String,
    client: reqwest::Client,
}

impl JevDecider {
    pub fn new(cfg: DecisionModelConfig, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()
            .unwrap_or_default();
        Self {
            cfg,
            api_key,
            client,
        }
    }

    /// Build from config. The key is `cfg.api_key`, then the `[llm]` key when
    /// `[llm]` is OpenRouter, then the variable named by `cfg.api_key_env`.
    pub fn from_config(cfg: &DecisionModelConfig, llm: &LlmConfig) -> anyhow::Result<Self> {
        Self::from_config_with_env(cfg, llm, |name| std::env::var(name).ok())
    }

    fn from_config_with_env(
        cfg: &DecisionModelConfig,
        llm: &LlmConfig,
        env: impl Fn(&str) -> Option<String>,
    ) -> anyhow::Result<Self> {
        let key = resolve_openrouter_key(cfg.api_key.as_ref(), llm, &cfg.api_key_env, env)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "decision model is enabled but has no API key: set \
                     [permissions.decision_model] api_key, an OpenRouter [llm] api_key, \
                     or {}",
                    cfg.api_key_env
                )
            })?;
        Ok(Self::new(cfg.clone(), key.expose().to_owned()))
    }

    async fn score(&self, req: &PermissionRequest<'_>) -> Result<Vec<(&'static str, f64)>, String> {
        let task = req
            .task_context
            .map(|t| truncate_chars(t, MAX_TASK_CHARS))
            .filter(|t| !t.trim().is_empty());

        let mut state = json!({
            "commands": [truncate_chars(&render_call(req).await, MAX_COMMAND_CHARS)],
            "project": project_dir(&req.ctx.agent_id),
            "agent": req.ctx.agent_id,
        });
        let mut questions = json!({
            "reversible": { "type": "noul", "instructions": REVERSIBLE_INSTRUCTIONS },
        });
        let mut asked = vec!["reversible"];
        if let Some(task) = task {
            state["task"] = json!(task);
            questions["serves_task"] =
                json!({ "type": "noul", "instructions": SERVES_TASK_INSTRUCTIONS });
            asked.push("serves_task");
        }

        let response = self
            .client
            .post(&self.cfg.base_url)
            .bearer_auth(&self.api_key)
            .json(&json!({ "model": self.cfg.model, "state": state, "questions": questions }))
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e.without_url()))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|e| format!("unreadable response: {}", e.without_url()))?;

        asked
            .into_iter()
            .map(|key| {
                let score = body["answers"][key]["noul"]
                    .as_f64()
                    .filter(|s| (0.0..=1.0).contains(s))
                    .ok_or_else(|| format!("missing or out-of-range score for '{key}'"))?;
                Ok((key, score))
            })
            .collect()
    }
}

#[async_trait]
impl ToolDecider for JevDecider {
    async fn decide(&self, req: &PermissionRequest<'_>) -> Decision {
        let scores = match self.score(req).await {
            Ok(scores) => scores,
            Err(reason) => {
                warn!(tool = req.tool_name, %reason, "decision model unavailable; failing closed");
                return Decision::new(
                    Verdict::Ask,
                    DECIDER,
                    format!("decision model unavailable ({reason})"),
                )
                .with_rule(self.cfg.model.clone());
            }
        };

        let threshold = f64::from(self.cfg.threshold);
        let min = scores.iter().map(|(_, s)| *s).fold(f64::INFINITY, f64::min);
        let decision = if let Some((key, score)) = scores.iter().find(|(_, s)| *s < threshold) {
            Decision::new(
                Verdict::Ask,
                DECIDER,
                format!(
                    "decision model scored {key}={score:.2}, below the {threshold:.2} threshold"
                ),
            )
        } else {
            Decision::new(
                Verdict::Allow,
                DECIDER,
                format!("decision model approved (lowest score {min:.2})"),
            )
        };
        decision
            .with_rule(self.cfg.model.clone())
            .with_confidence(min as f32)
    }
}

fn project_dir(agent_id: &str) -> String {
    runtime_workspace_dir()
        .unwrap_or_else(|| default_workspace_dir(agent_id))
        .to_string_lossy()
        .into_owned()
}

fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}… [truncated]", &s[..i]),
        None => s.to_string(),
    }
}

/// One-line description of the call. Network tools also state the destination
/// host and whether it is a private address.
async fn render_call(req: &PermissionRequest<'_>) -> String {
    match req.tool_name {
        "bash" => req.params["command"]
            .as_str()
            .map_or_else(|| req.params.to_string(), str::to_string),
        "web-fetch" => match req.params["url"].as_str() {
            Some(url) => format!("web-fetch GET {url} ({})", describe_destination(url).await),
            None => format!("web-fetch {}", req.params),
        },
        tool => format!("{tool} {}", req.params),
    }
}

async fn describe_destination(url: &str) -> String {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return "destination unknown".to_string();
    };
    let Some(host) = parsed.host_str() else {
        return "destination unknown".to_string();
    };
    let host = host.trim_matches(['[', ']']);
    let class = match classify_host(host, parsed.port_or_known_default().unwrap_or(80)).await {
        Some(true) => "private network address",
        Some(false) => "public address",
        None => "address unknown",
    };
    format!("host {host}, {class}")
}

/// `Some(true)` when any resolved address is private; `None` if it cannot be resolved.
async fn classify_host(host: &str, port: u16) -> Option<bool> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(is_private(ip));
    }
    let addrs = tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host((host, port)))
        .await
        .ok()?
        .ok()?;
    let mut any = false;
    let mut private = false;
    for addr in addrs {
        any = true;
        private |= is_private(addr.ip());
    }
    any.then_some(private)
}

/// Loopback, private, link-local, unique-local, unspecified or CGNAT.
fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use uuid::Uuid;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::Secret;
    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use assistant_core::types::llm::LlmProviderKind;

    use super::*;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::nil(),
            agent_id: "jev-test".into(),
            turn: 0,
            interface: Interface::Scheduler,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn decider(server: &MockServer, timeout_secs: u64) -> JevDecider {
        JevDecider::new(
            DecisionModelConfig {
                enabled: true,
                base_url: format!("{}/api/alpha/decisions", server.uri()),
                timeout_secs,
                ..Default::default()
            },
            "test-key".into(),
        )
    }

    // -- key resolution --

    fn openrouter_llm(key: Option<&str>) -> LlmConfig {
        LlmConfig {
            provider: LlmProviderKind::OpenRouter,
            api_key: key.map(Secret::new),
            ..LlmConfig::default()
        }
    }

    fn key_of(
        cfg: DecisionModelConfig,
        llm: &LlmConfig,
        env: impl Fn(&str) -> Option<String>,
    ) -> anyhow::Result<String> {
        JevDecider::from_config_with_env(&cfg, llm, env).map(|d| d.api_key)
    }

    #[test]
    fn config_api_key_wins() {
        let cfg = DecisionModelConfig {
            api_key: Some(Secret::new("k1")),
            ..Default::default()
        };
        let key = key_of(cfg, &openrouter_llm(Some("k2")), |_| Some("k3".into())).unwrap();
        assert_eq!(key, "k1", "decision_model.api_key comes first");
    }

    #[test]
    fn openrouter_chat_key_is_reused() {
        let key = key_of(
            DecisionModelConfig::default(),
            &openrouter_llm(Some("k2")),
            |_| None,
        )
        .unwrap();
        assert_eq!(key, "k2", "an OpenRouter [llm] key is reused");
    }

    #[test]
    fn api_key_env_is_the_last_resort() {
        let cfg = DecisionModelConfig {
            api_key_env: "JEV_KEY".into(),
            ..Default::default()
        };
        let llm = LlmConfig {
            provider: LlmProviderKind::Anthropic,
            api_key: Some(Secret::new("sk-ant")),
            ..LlmConfig::default()
        };
        let key = key_of(cfg.clone(), &llm, |name| {
            (name == "JEV_KEY").then(|| "k3".to_string())
        })
        .unwrap();
        assert_eq!(key, "k3", "the env var named by api_key_env is read last");

        let err = key_of(cfg, &llm, |_| None).unwrap_err().to_string();
        assert!(
            !err.contains("sk-ant"),
            "a non-OpenRouter key is never used or echoed: {err}"
        );
        assert!(
            err.contains("JEV_KEY") && err.contains("[llm]"),
            "the error names every source: {err}"
        );
    }

    fn answers(reversible: f64, serves_task: Option<f64>) -> Value {
        let mut a = json!({ "reversible": { "type": "noul", "noul": reversible } });
        if let Some(s) = serves_task {
            a["serves_task"] = json!({ "type": "noul", "noul": s });
        }
        json!({ "id": "x", "model": "typesafe/jev-1.13", "answers": a })
    }

    async fn mount(server: &MockServer, template: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/api/alpha/decisions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(template)
            .mount(server)
            .await;
    }

    async fn decide(d: &JevDecider, tool: &str, params: Value, task: Option<&str>) -> Decision {
        let ctx = ctx();
        d.decide(&PermissionRequest {
            tool_name: tool,
            params: &params,
            ctx: &ctx,
            requires_confirmation: false,
            task_context: task,
        })
        .await
    }

    async fn bash(d: &JevDecider, command: &str, task: Option<&str>) -> Decision {
        decide(d, "bash", json!({ "command": command }), task).await
    }

    #[tokio::test]
    async fn approves_when_every_score_reaches_the_threshold() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, Some(0.97))),
        )
        .await;
        let d = bash(&decider(&server, 5), "cargo test", Some("fix the build")).await;
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.decider, "decision-model");
        assert_eq!(d.rule.as_deref(), Some("typesafe/jev-1.13"));
        assert_eq!(d.confidence, Some(0.95));
    }

    #[tokio::test]
    async fn does_not_approve_below_the_threshold() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, Some(0.5))),
        )
        .await;
        let d = bash(&decider(&server, 5), "cargo test", Some("fix the build")).await;
        assert_eq!(d.verdict, Verdict::Ask);
        assert!(d.reason.contains("serves_task"), "{}", d.reason);
    }

    #[tokio::test]
    async fn fails_closed_on_http_errors_bad_bodies_and_out_of_range_scores() {
        for template in [
            ResponseTemplate::new(500),
            ResponseTemplate::new(200).set_body_string("not json"),
            ResponseTemplate::new(200).set_body_json(json!({ "answers": {} })),
            ResponseTemplate::new(200).set_body_json(answers(1.5, None)),
            ResponseTemplate::new(200).set_body_json(answers(-0.1, None)),
        ] {
            let server = MockServer::start().await;
            mount(&server, template).await;
            let d = bash(&decider(&server, 5), "ls", None).await;
            assert_eq!(d.verdict, Verdict::Ask, "{}", d.reason);
            assert!(d.confidence.is_none());
        }
    }

    #[tokio::test]
    async fn fails_closed_on_timeout() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200)
                .set_body_json(answers(0.99, None))
                .set_delay(Duration::from_secs(3)),
        )
        .await;
        let d = bash(&decider(&server, 1), "ls", None).await;
        assert_eq!(d.verdict, Verdict::Ask);
    }

    #[tokio::test]
    async fn fails_closed_when_the_server_is_unreachable() {
        let d = JevDecider::new(
            DecisionModelConfig {
                base_url: "http://127.0.0.1:1/api/alpha/decisions".into(),
                ..Default::default()
            },
            "k".into(),
        );
        assert_eq!(bash(&d, "ls", None).await.verdict, Verdict::Ask);
    }

    #[tokio::test]
    async fn request_carries_pinned_model_task_and_both_questions() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, Some(0.95))),
        )
        .await;
        bash(&decider(&server, 5), "cargo test", Some("fix the build")).await;

        let req = &server.received_requests().await.unwrap()[0];
        let body: Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(body["model"], "typesafe/jev-1.13");
        assert_eq!(body["state"]["task"], "fix the build");
        assert_eq!(body["state"]["agent"], "jev-test");
        assert!(
            body["state"]["commands"][0]
                .as_str()
                .unwrap()
                .contains("cargo test")
        );
        assert_eq!(body["questions"]["reversible"]["type"], "noul");
        assert_eq!(body["questions"]["serves_task"]["type"], "noul");
    }

    #[tokio::test]
    async fn serves_task_is_not_asked_without_a_task() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, None)),
        )
        .await;
        let d = bash(&decider(&server, 5), "ls", None).await;
        assert_eq!(d.verdict, Verdict::Allow);
        let body: Value =
            serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
        assert!(body["questions"].get("serves_task").is_none());
        assert!(body["state"].get("task").is_none());
    }

    #[tokio::test]
    async fn a_missing_answer_for_an_asked_question_is_not_approval() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.99, None)),
        )
        .await;
        let d = bash(&decider(&server, 5), "ls", Some("task")).await;
        assert_eq!(d.verdict, Verdict::Ask);
    }

    #[tokio::test]
    async fn web_fetch_requests_describe_the_destination() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, None)),
        )
        .await;
        decide(
            &decider(&server, 5),
            "web-fetch",
            json!({ "url": "http://192.168.1.20:8123/api/states" }),
            None,
        )
        .await;
        let body: Value =
            serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
        let command = body["state"]["commands"][0].as_str().unwrap();
        assert!(
            command.contains("http://192.168.1.20:8123/api/states"),
            "{command}"
        );
        assert!(command.contains("host 192.168.1.20"), "{command}");
        assert!(command.contains("private network address"), "{command}");
    }

    #[tokio::test]
    async fn public_destinations_are_not_flagged_private() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, None)),
        )
        .await;
        decide(
            &decider(&server, 5),
            "web-fetch",
            json!({ "url": "http://93.184.216.34/" }),
            None,
        )
        .await;
        let body: Value =
            serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
        let command = body["state"]["commands"][0].as_str().unwrap();
        assert!(command.contains("public address"), "{command}");
        assert!(!command.contains("private network"), "{command}");
    }

    #[tokio::test]
    async fn long_arguments_are_truncated_in_the_request() {
        let server = MockServer::start().await;
        mount(
            &server,
            ResponseTemplate::new(200).set_body_json(answers(0.95, None)),
        )
        .await;
        bash(&decider(&server, 5), &"x".repeat(50_000), None).await;
        let body: Value =
            serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
        assert!(body["state"]["commands"][0].as_str().unwrap().len() < 5_000);
    }
}
