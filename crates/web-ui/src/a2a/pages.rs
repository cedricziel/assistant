//! Server-side rendered HTML pages for agent management.
//!
//! Follows the same patterns as the trace/log pages: dark theme, sidebar
//! layout, `format!()` string assembly, `default_css()`.

use std::collections::HashMap;

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use assistant_a2a_json_schema::agent_card::*;

use super::agent_store::AgentStore;

// -- Shared state for the pages --

/// State required by the agent management pages.
#[derive(Clone)]
pub struct AgentPagesState {
    pub agent_store: AgentStore,
    pub base_url: String,
}

// -- Page handlers --

/// `GET /agents` -- Lists all registered local agents.
pub async fn list_agents(State(state): State<AgentPagesState>) -> Html<String> {
    let agents = state.agent_store.list().await.unwrap_or_default();
    let count = agents.len();

    let mut rows = String::new();
    for agent in &agents {
        let short_id = &agent.id[..8.min(agent.id.len())];
        let skill_count = agent.card.skills.len();
        let streaming = agent.card.capabilities.streaming.unwrap_or(false);
        let default_badge = if agent.is_default {
            "<span class=\"hdr-badge ok\">default</span>"
        } else {
            ""
        };
        let streaming_badge = if streaming {
            "<span class=\"badge\">streaming</span>"
        } else {
            ""
        };
        let interface_count = agent.card.supported_interfaces.len();

        rows.push_str(&format!(
            "<tr onclick=\"window.location='/agents/{id}'\">\
             <td><span class=\"trace-id\">{short_id}&hellip;</span></td>\
             <td><span class=\"primary\">{name}</span> {default_badge}</td>\
             <td>{version}</td>\
             <td>{skills} skill{skill_s}</td>\
             <td>{interfaces} interface{iface_s}</td>\
             <td>{streaming_badge}</td>\
             </tr>",
            id = html_escape(&agent.id),
            short_id = html_escape(short_id),
            name = html_escape(&agent.card.name),
            default_badge = default_badge,
            version = html_escape(&agent.card.version),
            skills = skill_count,
            skill_s = if skill_count == 1 { "" } else { "s" },
            interfaces = interface_count,
            iface_s = if interface_count == 1 { "" } else { "s" },
            streaming_badge = streaming_badge,
        ));
    }

    let table = if agents.is_empty() {
        "<p class=\"empty\">No agents registered yet. Create one to get started.</p>".to_string()
    } else {
        format!(
            "<table class=\"trace-table\">\
             <thead><tr>\
             <th>ID</th><th>Name</th><th>Version</th><th>Skills</th><th>Interfaces</th><th>Caps</th>\
             </tr></thead>\
             <tbody>{rows}</tbody></table>",
            rows = rows,
        )
    };

    let content = format!(
        "<div class=\"panel\">\
         <div class=\"panel-head\">\
         <div><h2>Local Agents</h2>\
         <p>Manage agent cards for your A2A-enabled agents.</p></div>\
         <span class=\"pill\">{count}</span>\
         </div>\
         {table}\
         <div style=\"margin-top:1.25rem\">\
         <a href=\"/agents/new\" class=\"action-btn\">+ New Agent</a>\
         </div>\
         </div>",
        count = count,
        table = table,
    );

    let sidebar = render_sidebar("agents");
    let body = page_shell("Agents", &sidebar, &content);
    Html(body)
}

/// `GET /agents/new` -- Form to create a new agent.
pub async fn new_agent_form(State(_state): State<AgentPagesState>) -> Html<String> {
    let form = render_agent_form(None, "Create Agent", "/agents", "Create");
    let sidebar = render_sidebar("agents");
    let body = page_shell("New Agent", &sidebar, &form);
    Html(body)
}

/// `POST /agents` -- Creates a new agent from form data.
pub async fn create_agent(
    State(state): State<AgentPagesState>,
    Form(form): Form<AgentFormData>,
) -> Response {
    let set_default = form.is_default.is_some();
    let card = form.into_agent_card(&state.base_url);
    match state.agent_store.register(card, set_default).await {
        Ok(id) => Redirect::to(&format!("/agents/{id}")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /agents/:id` -- Agent detail page.
pub async fn show_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let agent = state
        .agent_store
        .get(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Agent '{id}' not found")))?;

    let card = &agent.card;

    // -- Header --
    let default_badge = if agent.is_default {
        " <span class=\"hdr-badge ok\">default</span>"
    } else {
        ""
    };

    let header = format!(
        "<div class=\"trace-header-bar\">\
         <a class=\"hdr-back\" href=\"/agents\">&larr; Agents</a>\
         <span class=\"hdr-sep\">|</span>\
         <span class=\"hdr-trace-id\">{short_id}&hellip;</span>\
         <span class=\"hdr-svc\">{name}</span>{default_badge}\
         <span class=\"hdr-dur\">v{version}</span>\
         </div>",
        short_id = html_escape(&id[..8.min(id.len())]),
        name = html_escape(&card.name),
        default_badge = default_badge,
        version = html_escape(&card.version),
    );

    // -- Description --
    let desc_section = format!(
        "<div class=\"agent-section\">\
         <h3>Description</h3>\
         <p>{desc}</p>\
         </div>",
        desc = html_escape(&card.description),
    );

    // -- Interfaces --
    let mut iface_rows = String::new();
    for iface in &card.supported_interfaces {
        iface_rows.push_str(&format!(
            "<tr>\
             <td><span class=\"badge\">{binding}</span></td>\
             <td>{url}</td>\
             <td>{version}</td>\
             </tr>",
            binding = html_escape(&iface.protocol_binding),
            url = html_escape(&iface.url),
            version = html_escape(&iface.protocol_version),
        ));
    }
    let iface_section = format!(
        "<div class=\"agent-section\">\
         <h3>Interfaces</h3>\
         <table class=\"trace-table\">\
         <thead><tr><th>Binding</th><th>URL</th><th>Protocol</th></tr></thead>\
         <tbody>{rows}</tbody></table>\
         </div>",
        rows = iface_rows,
    );

    // -- Capabilities --
    let caps = &card.capabilities;
    let cap_items = format!(
        "<ul class=\"cap-list\">\
         <li>Streaming: {streaming}</li>\
         <li>Push Notifications: {push}</li>\
         <li>Extended Card: {ext_card}</li>\
         <li>Extensions: {ext_count}</li>\
         </ul>",
        streaming = bool_badge(caps.streaming),
        push = bool_badge(caps.push_notifications),
        ext_card = bool_badge(caps.extended_agent_card),
        ext_count = caps.extensions.len(),
    );
    let cap_section = format!(
        "<div class=\"agent-section\">\
         <h3>Capabilities</h3>\
         {items}\
         </div>",
        items = cap_items,
    );

    // -- Skills --
    let mut skill_cards = String::new();
    for skill in &card.skills {
        let tags: String = skill
            .tags
            .iter()
            .map(|t| format!("<span class=\"badge muted\">{}</span>", html_escape(t)))
            .collect::<Vec<_>>()
            .join(" ");

        let examples = if skill.examples.is_empty() {
            String::new()
        } else {
            let ex: String = skill
                .examples
                .iter()
                .map(|e| format!("<li>{}</li>", html_escape(e)))
                .collect();
            format!("<div class=\"skill-examples\"><strong>Examples:</strong><ul>{ex}</ul></div>")
        };

        skill_cards.push_str(&format!(
            "<div class=\"skill-card\">\
             <div class=\"skill-head\">\
             <span class=\"primary\">{name}</span>\
             <span class=\"trace-id\">{id}</span>\
             </div>\
             <p>{desc}</p>\
             <div class=\"skill-tags\">{tags}</div>\
             {examples}\
             </div>",
            name = html_escape(&skill.name),
            id = html_escape(&skill.id),
            desc = html_escape(&skill.description),
            tags = tags,
            examples = examples,
        ));
    }

    let skill_section = if card.skills.is_empty() {
        "<div class=\"agent-section\"><h3>Skills</h3>\
         <p class=\"empty\">No skills defined.</p></div>"
            .to_string()
    } else {
        format!(
            "<div class=\"agent-section\">\
             <h3>Skills <span class=\"pill\">{count}</span></h3>\
             <div class=\"skill-grid\">{cards}</div>\
             </div>",
            count = card.skills.len(),
            cards = skill_cards,
        )
    };

    // -- I/O Modes --
    let input_modes = card
        .default_input_modes
        .iter()
        .map(|m| format!("<span class=\"badge\">{}</span>", html_escape(m)))
        .collect::<Vec<_>>()
        .join(" ");
    let output_modes = card
        .default_output_modes
        .iter()
        .map(|m| format!("<span class=\"badge\">{}</span>", html_escape(m)))
        .collect::<Vec<_>>()
        .join(" ");
    let modes_section = format!(
        "<div class=\"agent-section\">\
         <h3>Default I/O Modes</h3>\
         <p><strong>Input:</strong> {input}</p>\
         <p><strong>Output:</strong> {output}</p>\
         </div>",
        input = if input_modes.is_empty() {
            "<span class=\"muted\">none</span>".to_string()
        } else {
            input_modes
        },
        output = if output_modes.is_empty() {
            "<span class=\"muted\">none</span>".to_string()
        } else {
            output_modes
        },
    );

    // -- Provider --
    let provider_section = if let Some(provider) = &card.provider {
        format!(
            "<div class=\"agent-section\">\
             <h3>Provider</h3>\
             <p><strong>{org}</strong> &mdash; <a href=\"{url}\">{url}</a></p>\
             </div>",
            org = html_escape(&provider.organization),
            url = html_escape(&provider.url),
        )
    } else {
        String::new()
    };

    // -- Actions --
    let actions = format!(
        "<div class=\"agent-actions\">\
         <a href=\"/agents/{id}/edit\" class=\"action-btn\">Edit</a>\
         {set_default_btn}\
         <form method=\"POST\" action=\"/agents/{id}/delete\" style=\"display:inline\" \
               onsubmit=\"return confirm('Delete this agent?')\">\
         <button type=\"submit\" class=\"action-btn danger\">Delete</button>\
         </form>\
         <a href=\"/agents/{id}/card.json\" class=\"action-btn secondary\" \
            target=\"_blank\">View JSON</a>\
         </div>",
        id = html_escape(&agent.id),
        set_default_btn = if agent.is_default {
            String::new()
        } else {
            format!(
                "<form method=\"POST\" action=\"/agents/{id}/set-default\" style=\"display:inline\">\
                 <button type=\"submit\" class=\"action-btn secondary\">Set as Default</button>\
                 </form>",
                id = html_escape(&agent.id),
            )
        },
    );

    let detail = format!(
        "<div class=\"trace-detail\">\
         {header}\
         {desc}\
         {actions}\
         {iface}\
         {caps}\
         {modes}\
         {skills}\
         {provider}\
         </div>",
        header = header,
        desc = desc_section,
        actions = actions,
        iface = iface_section,
        caps = cap_section,
        modes = modes_section,
        skills = skill_section,
        provider = provider_section,
    );

    let sidebar = render_sidebar("agents");
    let body = page_shell(&format!("Agent: {}", card.name), &sidebar, &detail);
    Ok(Html(body))
}

/// `GET /agents/:id/card.json` -- Raw agent card JSON.
pub async fn show_agent_card_json(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Response {
    match state.agent_store.get(&id).await {
        Some(agent) => axum::Json(agent.card).into_response(),
        None => (StatusCode::NOT_FOUND, "Agent not found".to_string()).into_response(),
    }
}

/// `GET /agents/:id/edit` -- Edit form for an existing agent.
pub async fn edit_agent_form(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let agent = state
        .agent_store
        .get(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Agent '{id}' not found")))?;

    let form = render_agent_form(
        Some(&agent),
        "Edit Agent",
        &format!("/agents/{id}/edit"),
        "Save Changes",
    );
    let sidebar = render_sidebar("agents");
    let body = page_shell(&format!("Edit: {}", agent.card.name), &sidebar, &form);
    Ok(Html(body))
}

/// `POST /agents/:id/edit` -- Updates an agent from form data.
pub async fn update_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
    Form(form): Form<AgentFormData>,
) -> Response {
    let card = form.into_agent_card(&state.base_url);
    if state.agent_store.update(&id, card).await {
        Redirect::to(&format!("/agents/{id}")).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Agent not found".to_string()).into_response()
    }
}

/// `POST /agents/:id/delete` -- Deletes an agent.
pub async fn delete_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Response {
    state.agent_store.remove(&id).await;
    Redirect::to("/agents").into_response()
}

/// `POST /agents/:id/set-default` -- Sets an agent as the default.
pub async fn set_default_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Response {
    state.agent_store.set_default(&id).await;
    Redirect::to(&format!("/agents/{id}")).into_response()
}

// -- Form data --

/// Form fields for creating/editing an agent.
#[derive(Debug, Deserialize)]
pub struct AgentFormData {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub protocol_binding: String,
    #[serde(default)]
    pub documentation_url: String,
    #[serde(default)]
    pub streaming: Option<String>,
    #[serde(default)]
    pub is_default: Option<String>,
    #[serde(default)]
    pub input_modes: String,
    #[serde(default)]
    pub output_modes: String,
    #[serde(default)]
    pub provider_org: String,
    #[serde(default)]
    pub provider_url: String,
    #[serde(default)]
    pub skills_json: String,
}

impl AgentFormData {
    fn into_agent_card(self, default_url: &str) -> AgentCard {
        let url = if self.url.is_empty() {
            default_url.to_string()
        } else {
            self.url
        };
        let binding = if self.protocol_binding.is_empty() {
            "HTTP+JSON".to_string()
        } else {
            self.protocol_binding
        };

        let provider = if self.provider_org.is_empty() {
            None
        } else {
            Some(AgentProvider {
                organization: self.provider_org,
                url: self.provider_url,
            })
        };

        let skills: Vec<AgentSkill> = if self.skills_json.trim().is_empty() {
            vec![]
        } else {
            serde_json::from_str(&self.skills_json).unwrap_or_default()
        };

        let input_modes: Vec<String> = self
            .input_modes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let output_modes: Vec<String> = self
            .output_modes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        AgentCard {
            name: self.name,
            description: self.description,
            supported_interfaces: vec![AgentInterface {
                url,
                protocol_binding: binding,
                tenant: None,
                protocol_version: "1.0".to_string(),
            }],
            provider,
            version: self.version,
            documentation_url: if self.documentation_url.is_empty() {
                None
            } else {
                Some(self.documentation_url)
            },
            capabilities: AgentCapabilities {
                streaming: Some(self.streaming.is_some()),
                push_notifications: Some(false),
                extensions: vec![],
                extended_agent_card: None,
            },
            security_schemes: HashMap::new(),
            security_requirements: vec![],
            default_input_modes: if input_modes.is_empty() {
                vec!["text/plain".to_string()]
            } else {
                input_modes
            },
            default_output_modes: if output_modes.is_empty() {
                vec!["text/plain".to_string()]
            } else {
                output_modes
            },
            skills,
            signatures: vec![],
            icon_url: None,
        }
    }
}

// -- Rendering helpers --

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn bool_badge(val: Option<bool>) -> &'static str {
    match val {
        Some(true) => "<span class=\"status-ok\">&#x2713;</span>",
        Some(false) => "<span class=\"muted\">&#x2717;</span>",
        None => "<span class=\"muted\">&mdash;</span>",
    }
}

fn render_sidebar(active: &str) -> String {
    let items = [
        ("Traces", "/traces"),
        ("Logs", "/logs"),
        ("Agents", "/agents"),
        ("Webhooks", "/webhooks"),
    ];
    let mut links = String::new();
    for (label, href) in &items {
        let class = if *label.to_ascii_lowercase() == *active {
            "facet-link active"
        } else {
            "facet-link"
        };
        links.push_str(&format!(
            "<li><a class=\"{class}\" href=\"{href}\"><span>{label}</span></a></li>",
            class = class,
            href = href,
            label = label,
        ));
    }

    format!(
        "<div class=\"sidebar-inner\">\
         <div class=\"brand\"><p>assistant</p><h2>Agent Manager</h2></div>\
         <div class=\"facet-group\">\
         <h3>Navigation</h3>\
         <ul>{links}</ul>\
         </div>\
         </div>",
        links = links,
    )
}

fn page_shell(title: &str, sidebar: &str, content: &str) -> String {
    format!(
        "<!DOCTYPE html>\
         <html><head>\
         <meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{title} - Assistant</title>\
         <style>{css}\n{extra_css}</style>\
         </head><body>\
         <div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{content}</main>\
         </div></body></html>",
        title = html_escape(title),
        css = crate::default_css(),
        extra_css = agents_css(),
        sidebar = sidebar,
        content = content,
    )
}

fn render_agent_form(
    agent: Option<&super::agent_store::RegisteredAgent>,
    heading: &str,
    action: &str,
    submit_label: &str,
) -> String {
    let card = agent.map(|a| &a.card);

    let name = card.map(|c| c.name.as_str()).unwrap_or("");
    let desc = card.map(|c| c.description.as_str()).unwrap_or("");
    let version = card.map(|c| c.version.as_str()).unwrap_or("1.0.0");
    let url = card
        .and_then(|c| c.supported_interfaces.first().map(|i| i.url.as_str()))
        .unwrap_or("");
    let binding = card
        .and_then(|c| {
            c.supported_interfaces
                .first()
                .map(|i| i.protocol_binding.as_str())
        })
        .unwrap_or("HTTP+JSON");
    let doc_url = card
        .and_then(|c| c.documentation_url.as_deref())
        .unwrap_or("");
    let streaming = card.and_then(|c| c.capabilities.streaming).unwrap_or(true);
    let input_modes = card
        .map(|c| c.default_input_modes.join(", "))
        .unwrap_or_else(|| "text/plain".to_string());
    let output_modes = card
        .map(|c| c.default_output_modes.join(", "))
        .unwrap_or_else(|| "text/plain".to_string());
    let provider_org = card
        .and_then(|c| c.provider.as_ref().map(|p| p.organization.as_str()))
        .unwrap_or("");
    let provider_url = card
        .and_then(|c| c.provider.as_ref().map(|p| p.url.as_str()))
        .unwrap_or("");
    let skills_json = card
        .map(|c| serde_json::to_string_pretty(&c.skills).unwrap_or_default())
        .unwrap_or_else(|| "[]".to_string());

    let streaming_checked = if streaming { "checked" } else { "" };

    format!(
        "<div class=\"panel\">\
         <div class=\"panel-head\"><h2>{heading}</h2></div>\
         <form method=\"POST\" action=\"{action}\" class=\"agent-form\">\
         <div class=\"form-group\">\
           <label>Name *</label>\
           <input type=\"text\" name=\"name\" value=\"{name}\" required placeholder=\"My Agent\">\
         </div>\
         <div class=\"form-group\">\
           <label>Description *</label>\
           <textarea name=\"description\" required rows=\"3\" \
                     placeholder=\"What does this agent do?\">{desc}</textarea>\
         </div>\
         <div class=\"form-row\">\
           <div class=\"form-group\">\
             <label>Version *</label>\
             <input type=\"text\" name=\"version\" value=\"{version}\" required placeholder=\"1.0.0\">\
           </div>\
           <div class=\"form-group\">\
             <label>Protocol Binding</label>\
             <input type=\"text\" name=\"protocol_binding\" value=\"{binding}\" \
                    placeholder=\"HTTP+JSON\">\
           </div>\
         </div>\
         <div class=\"form-group\">\
           <label>Interface URL</label>\
           <input type=\"text\" name=\"url\" value=\"{url}\" \
                  placeholder=\"https://example.com/a2a\">\
         </div>\
         <div class=\"form-group\">\
           <label>Documentation URL</label>\
           <input type=\"text\" name=\"documentation_url\" value=\"{doc_url}\" \
                  placeholder=\"https://docs.example.com\">\
         </div>\
         <div class=\"form-row\">\
           <div class=\"form-group\">\
             <label>Input Modes (comma-separated)</label>\
             <input type=\"text\" name=\"input_modes\" value=\"{input_modes}\" \
                    placeholder=\"text/plain\">\
           </div>\
           <div class=\"form-group\">\
             <label>Output Modes (comma-separated)</label>\
             <input type=\"text\" name=\"output_modes\" value=\"{output_modes}\" \
                    placeholder=\"text/plain\">\
           </div>\
         </div>\
         <div class=\"form-row\">\
           <div class=\"form-group\">\
             <label>Provider Organization</label>\
             <input type=\"text\" name=\"provider_org\" value=\"{provider_org}\" \
                    placeholder=\"Acme Inc.\">\
           </div>\
           <div class=\"form-group\">\
             <label>Provider URL</label>\
             <input type=\"text\" name=\"provider_url\" value=\"{provider_url}\" \
                    placeholder=\"https://acme.com\">\
           </div>\
         </div>\
         <div class=\"form-group\">\
           <label class=\"checkbox-label\">\
             <input type=\"checkbox\" name=\"streaming\" value=\"on\" {streaming_checked}>\
             Supports streaming\
           </label>\
         </div>\
         <div class=\"form-group\">\
           <label>Skills (JSON array)</label>\
           <textarea name=\"skills_json\" rows=\"10\" class=\"code-input\" \
                     placeholder='[{{\"id\":\"...\",\"name\":\"...\",\"description\":\"...\",\"tags\":[\"...\"]}}]'\
           >{skills_json}</textarea>\
         </div>\
         <div class=\"form-actions\">\
           <button type=\"submit\" class=\"action-btn\">{submit_label}</button>\
           <a href=\"/agents\" class=\"action-btn secondary\">Cancel</a>\
         </div>\
         </form>\
         </div>",
        heading = html_escape(heading),
        action = html_escape(action),
        name = html_escape(name),
        desc = html_escape(desc),
        version = html_escape(version),
        url = html_escape(url),
        binding = html_escape(binding),
        doc_url = html_escape(doc_url),
        streaming_checked = streaming_checked,
        input_modes = html_escape(&input_modes),
        output_modes = html_escape(&output_modes),
        provider_org = html_escape(provider_org),
        provider_url = html_escape(provider_url),
        skills_json = html_escape(&skills_json),
        submit_label = html_escape(submit_label),
    )
}

/// Additional CSS for agent management pages.
fn agents_css() -> &'static str {
    r#"
    .agent-form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
    }
    .form-group label {
        font-size: 0.85rem;
        color: #8aa5d8;
        text-transform: uppercase;
        letter-spacing: 0.06em;
    }
    .form-group input[type=text],
    .form-group textarea {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 8px;
        color: #e5e9f0;
        padding: 0.5rem 0.75rem;
        font-size: 0.9rem;
        font-family: inherit;
        width: 100%;
    }
    .form-group textarea {
        resize: vertical;
    }
    .code-input {
        font-family: ui-monospace, monospace !important;
        font-size: 0.82rem !important;
    }
    .form-row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }
    .form-actions {
        display: flex;
        gap: 0.75rem;
        margin-top: 0.5rem;
    }
    .checkbox-label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
    }
    .checkbox-label input[type=checkbox] {
        accent-color: #6ec6ff;
        width: 16px;
        height: 16px;
    }
    .action-btn {
        display: inline-block;
        background: linear-gradient(135deg, #64cafe, #8b5dff);
        border: none;
        border-radius: 8px;
        color: #050b16;
        padding: 0.5rem 1.2rem;
        font-weight: 600;
        font-size: 0.9rem;
        cursor: pointer;
        text-decoration: none;
        text-align: center;
    }
    .action-btn.secondary {
        background: rgba(255,255,255,0.08);
        color: #c2d6f0;
    }
    .action-btn.danger {
        background: rgba(248, 113, 113, 0.25);
        color: #ffb4b4;
    }
    .agent-actions {
        display: flex;
        gap: 0.75rem;
        flex-wrap: wrap;
        margin: 1.25rem 0;
        padding-bottom: 1.25rem;
        border-bottom: 1px solid #0f1f36;
    }
    .agent-section {
        margin: 1.25rem 0;
    }
    .agent-section h3 {
        margin: 0 0 0.6rem;
        color: #8aa5d8;
        font-size: 0.9rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }
    .agent-section p {
        margin: 0.3rem 0;
    }
    .cap-list {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.3rem;
    }
    .cap-list li {
        font-size: 0.9rem;
    }
    .skill-grid {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    .skill-card {
        background: #030a15;
        border: 1px solid #0f1f36;
        border-radius: 12px;
        padding: 1rem;
    }
    .skill-head {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 0.4rem;
    }
    .skill-card p {
        margin: 0 0 0.5rem;
        color: #a0bfe0;
        font-size: 0.9rem;
    }
    .skill-tags {
        display: flex;
        gap: 0.4rem;
        flex-wrap: wrap;
    }
    .skill-examples {
        margin-top: 0.5rem;
        font-size: 0.85rem;
        color: #8ba2c6;
    }
    .skill-examples ul {
        margin: 0.2rem 0 0 1.2rem;
        padding: 0;
    }
    .skill-examples li {
        margin-bottom: 0.15rem;
    }
    @media (max-width: 640px) {
        .form-row {
            grid-template-columns: 1fr;
        }
    }
    "#
}
