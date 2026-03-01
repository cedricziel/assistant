//! Server-side rendered HTML pages for agent management.
//!
//! All HTML is rendered via Askama templates under `templates/agents/`.

use std::collections::HashMap;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use assistant_a2a_json_schema::agent_card::*;

use super::agent_store::AgentStore;
use crate::common::{render_template, StaticUrls};

// -- Shared state for the pages --

/// State required by the agent management pages.
#[derive(Clone)]
pub struct AgentPagesState {
    pub agent_store: AgentStore,
    pub base_url: String,
}

// -- View models -------------------------------------------------------------

/// A row in the agent list table.
struct AgentRowView {
    id: String,
    short_id: String,
    name: String,
    version: String,
    skill_count: usize,
    interface_count: usize,
    streaming: bool,
    is_default: bool,
}

/// An interface row in the agent detail page.
struct InterfaceRowView {
    protocol_binding: String,
    url: String,
    protocol_version: String,
}

/// A skill card in the agent detail page.
struct SkillCardView {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
    examples: Vec<String>,
}

/// Provider info for the agent detail page.
struct ProviderView {
    organization: String,
    url: String,
}

/// Pre-populated form field values for create/edit.
struct AgentFormValues {
    name: String,
    desc: String,
    version: String,
    url: String,
    binding: String,
    doc_url: String,
    streaming_checked: bool,
    input_modes: String,
    output_modes: String,
    provider_org: String,
    provider_url: String,
    skills_json: String,
}

// -- Templates ---------------------------------------------------------------

/// Agent list page (extends base.html).
#[derive(Template)]
#[template(path = "agents/list.html")]
struct AgentsListTemplate {
    active_page: &'static str,
    agents: Vec<AgentRowView>,
    count: usize,
}

impl StaticUrls for AgentsListTemplate {}

/// Agent detail page (extends base.html).
#[derive(Template)]
#[template(path = "agents/detail.html")]
struct AgentDetailTemplate {
    active_page: &'static str,
    agent_id: String,
    agent_name: String,
    short_id: String,
    version: String,
    description: String,
    is_default: bool,
    interfaces: Vec<InterfaceRowView>,
    streaming_badge: &'static str,
    push_badge: &'static str,
    ext_card_badge: &'static str,
    extension_count: usize,
    input_modes: Vec<String>,
    output_modes: Vec<String>,
    skills: Vec<SkillCardView>,
    provider: Option<ProviderView>,
}

impl StaticUrls for AgentDetailTemplate {}

/// Agent create/edit form page (extends base.html).
#[derive(Template)]
#[template(path = "agents/form.html")]
struct AgentFormTemplate {
    active_page: &'static str,
    page_title: String,
    heading: String,
    action: String,
    submit_label: String,
    name: String,
    desc: String,
    version: String,
    url: String,
    binding: String,
    doc_url: String,
    streaming_checked: bool,
    input_modes: String,
    output_modes: String,
    provider_org: String,
    provider_url: String,
    skills_json: String,
}

impl StaticUrls for AgentFormTemplate {}

// -- Page handlers -----------------------------------------------------------

/// `GET /agents` -- Lists all registered local agents.
pub async fn list_agents(
    State(state): State<AgentPagesState>,
) -> Result<Response, (StatusCode, String)> {
    let agents = state
        .agent_store
        .list()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let count = agents.len();

    let rows: Vec<AgentRowView> = agents
        .iter()
        .map(|agent| AgentRowView {
            id: agent.id.clone(),
            short_id: agent.id[..8.min(agent.id.len())].to_string(),
            name: agent.card.name.clone(),
            version: agent.card.version.clone(),
            skill_count: agent.card.skills.len(),
            interface_count: agent.card.supported_interfaces.len(),
            streaming: agent.card.capabilities.streaming.unwrap_or(false),
            is_default: agent.is_default,
        })
        .collect();

    let tmpl = AgentsListTemplate {
        active_page: "agents",
        agents: rows,
        count,
    };
    Ok(render_template(tmpl))
}

/// `GET /agents/new` -- Form to create a new agent.
pub async fn new_agent_form(State(_state): State<AgentPagesState>) -> Response {
    let vals = build_form_values(None);
    let tmpl = AgentFormTemplate {
        active_page: "agents",
        page_title: "New Agent".to_string(),
        heading: "Create Agent".to_string(),
        action: "/agents".to_string(),
        submit_label: "Create".to_string(),
        name: vals.name,
        desc: vals.desc,
        version: vals.version,
        url: vals.url,
        binding: vals.binding,
        doc_url: vals.doc_url,
        streaming_checked: vals.streaming_checked,
        input_modes: vals.input_modes,
        output_modes: vals.output_modes,
        provider_org: vals.provider_org,
        provider_url: vals.provider_url,
        skills_json: vals.skills_json,
    };
    render_template(tmpl)
}

/// `POST /agents` -- Creates a new agent from form data.
pub async fn create_agent(
    State(state): State<AgentPagesState>,
    Form(form): Form<AgentFormData>,
) -> Response {
    let set_default = form.is_default.is_some();
    let card = match form.into_agent_card(&state.base_url) {
        Ok(card) => card,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };
    match state.agent_store.register(card, set_default).await {
        Ok(id) => Redirect::to(&format!("/agents/{id}")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /agents/:id` -- Agent detail page.
pub async fn show_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let agent = state
        .agent_store
        .get(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Agent '{id}' not found")))?;

    let card = &agent.card;
    let caps = &card.capabilities;

    let interfaces: Vec<InterfaceRowView> = card
        .supported_interfaces
        .iter()
        .map(|iface| InterfaceRowView {
            protocol_binding: iface.protocol_binding.clone(),
            url: iface.url.clone(),
            protocol_version: iface.protocol_version.clone(),
        })
        .collect();

    let skills: Vec<SkillCardView> = card
        .skills
        .iter()
        .map(|skill| SkillCardView {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            tags: skill.tags.clone(),
            examples: skill.examples.clone(),
        })
        .collect();

    let provider = card.provider.as_ref().map(|p| ProviderView {
        organization: p.organization.clone(),
        url: p.url.clone(),
    });

    let tmpl = AgentDetailTemplate {
        active_page: "agents",
        agent_id: agent.id.clone(),
        agent_name: card.name.clone(),
        short_id: id[..8.min(id.len())].to_string(),
        version: card.version.clone(),
        description: card.description.clone(),
        is_default: agent.is_default,
        interfaces,
        streaming_badge: bool_badge(caps.streaming),
        push_badge: bool_badge(caps.push_notifications),
        ext_card_badge: bool_badge(caps.extended_agent_card),
        extension_count: caps.extensions.len(),
        input_modes: card.default_input_modes.clone(),
        output_modes: card.default_output_modes.clone(),
        skills,
        provider,
    };
    Ok(render_template(tmpl))
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
) -> Result<Response, (StatusCode, String)> {
    let agent = state
        .agent_store
        .get(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, format!("Agent '{id}' not found")))?;

    let vals = build_form_values(Some(&agent));
    let tmpl = AgentFormTemplate {
        active_page: "agents",
        page_title: format!("Edit: {}", agent.card.name),
        heading: "Edit Agent".to_string(),
        action: format!("/agents/{id}/edit"),
        submit_label: "Save Changes".to_string(),
        name: vals.name,
        desc: vals.desc,
        version: vals.version,
        url: vals.url,
        binding: vals.binding,
        doc_url: vals.doc_url,
        streaming_checked: vals.streaming_checked,
        input_modes: vals.input_modes,
        output_modes: vals.output_modes,
        provider_org: vals.provider_org,
        provider_url: vals.provider_url,
        skills_json: vals.skills_json,
    };
    Ok(render_template(tmpl))
}

/// `POST /agents/:id/edit` -- Updates an agent from form data.
pub async fn update_agent(
    State(state): State<AgentPagesState>,
    Path(id): Path<String>,
    Form(form): Form<AgentFormData>,
) -> Response {
    let card = match form.into_agent_card(&state.base_url) {
        Ok(card) => card,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };
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

// -- Form data ---------------------------------------------------------------

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
    fn into_agent_card(self, default_url: &str) -> Result<AgentCard, String> {
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
            match serde_json::from_str(&self.skills_json) {
                Ok(skills) => skills,
                Err(e) => return Err(format!("Invalid skills JSON: {e}")),
            }
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

        Ok(AgentCard {
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
        })
    }
}

// -- Rendering helpers -------------------------------------------------------

fn bool_badge(val: Option<bool>) -> &'static str {
    match val {
        Some(true) => "<span class=\"status-ok\">&#x2713;</span>",
        Some(false) => "<span class=\"muted\">&#x2717;</span>",
        None => "<span class=\"muted\">&mdash;</span>",
    }
}

/// Build pre-populated form field values from an optional existing agent.
fn build_form_values(agent: Option<&super::agent_store::RegisteredAgent>) -> AgentFormValues {
    let card = agent.map(|a| &a.card);

    AgentFormValues {
        name: card.map(|c| c.name.clone()).unwrap_or_default(),
        desc: card.map(|c| c.description.clone()).unwrap_or_default(),
        version: card
            .map(|c| c.version.clone())
            .unwrap_or_else(|| "1.0.0".to_string()),
        url: card
            .and_then(|c| c.supported_interfaces.first().map(|i| i.url.clone()))
            .unwrap_or_default(),
        binding: card
            .and_then(|c| {
                c.supported_interfaces
                    .first()
                    .map(|i| i.protocol_binding.clone())
            })
            .unwrap_or_else(|| "HTTP+JSON".to_string()),
        doc_url: card
            .and_then(|c| c.documentation_url.clone())
            .unwrap_or_default(),
        streaming_checked: card.and_then(|c| c.capabilities.streaming).unwrap_or(true),
        input_modes: card
            .map(|c| c.default_input_modes.join(", "))
            .unwrap_or_else(|| "text/plain".to_string()),
        output_modes: card
            .map(|c| c.default_output_modes.join(", "))
            .unwrap_or_else(|| "text/plain".to_string()),
        provider_org: card
            .and_then(|c| c.provider.as_ref().map(|p| p.organization.clone()))
            .unwrap_or_default(),
        provider_url: card
            .and_then(|c| c.provider.as_ref().map(|p| p.url.clone()))
            .unwrap_or_default(),
        skills_json: card
            .map(|c| serde_json::to_string_pretty(&c.skills).unwrap_or_default())
            .unwrap_or_else(|| "[]".to_string()),
    }
}
