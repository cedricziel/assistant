//! OpenAPI spec generation and serving.
//!
//! Exposes:
//! - `GET /api/openapi.json` — machine-readable OpenAPI 3.1 spec
//! - `GET /api/docs` — Swagger UI

use assistant_a2a_json_schema::{
    agent_card::{
        AgentCapabilities, AgentCard, AgentCardSignature, AgentExtension, AgentInterface,
        AgentProvider, AgentSkill,
    },
    requests::{CancelTaskRequest, CreateTaskPushNotificationConfigRequest, SendMessageRequest},
    responses::{
        ListTaskPushNotificationConfigsResponse, ListTasksResponse, SendMessageResponse,
        StreamResponse,
    },
    security::{
        ApiKeySecurityScheme, AuthorizationCodeOAuthFlow, ClientCredentialsOAuthFlow,
        DeviceCodeOAuthFlow, HttpAuthSecurityScheme, ImplicitOAuthFlow, MutualTlsSecurityScheme,
        OAuth2SecurityScheme, OAuthFlows, OpenIdConnectSecurityScheme, PasswordOAuthFlow,
        SecurityRequirement, SecurityScheme as A2ASecurityScheme,
    },
    types::{
        Artifact, AuthenticationInfo, Message, Part, PushNotificationConfig, Role,
        SendMessageConfiguration, StringList, Task, TaskArtifactUpdateEvent,
        TaskPushNotificationConfig, TaskState, TaskStatus, TaskStatusUpdateEvent,
    },
};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::a2a::handlers;
use crate::api::push::{SubscribeRequest, UnsubscribeRequest, VapidKeyResponse};
use crate::api::{
    agents::{AgentDetail, AgentSummary, RegisterAgentRequest, UpdateAgentRequest},
    analytics::{
        AnalyticsSummaryResponse, ModelUsageResponse, TimeSeriesResponse, ToolUsageResponse,
    },
    logs::LogEntryResponse,
    personas::{
        AddSkillAccessRequest, CreatePersonaRequest, PersonaDetail, PersonaFileContent,
        PersonaFileSlot, PersonaSkillAccess, PersonaSummary, SetActivePersonaRequest,
        SetSkillAccessModeRequest, WritePersonaFileRequest,
    },
    skills::{CreateSkillRequest, SkillDetail, SkillEntryResponse, UpdateSkillRequest},
    traces::{SpanEntryResponse, TraceDetailResponse, TraceSummaryResponse},
    webhooks::{
        CreateWebhookRequest, RotateSecretResponse, UpdateWebhookRequest, VerifyWebhookResponse,
        WebhookResponse,
    },
    workflows::{
        WorkflowDetail, WorkflowRunDetail, WorkflowRunPreview, WorkflowRunStep, WorkflowRunSummary,
        WorkflowSummary, WorkflowUpsertRequest, WorkflowWebhookSecrets,
    },
    AttachmentMetaResponse, ConversationDetail, ConversationSummary, CreateConversationRequest,
    MessageSummary, SendMessageRequest as ApiSendMessageRequest, ServerCapabilities,
    StreamRunEventsQuery, ToolCallSummary, UpdateConversationRequest,
};

/// Adds the Bearer token security scheme to the OpenAPI components.
struct BearerTokenSecurityAddon;

impl Modify for BearerTokenSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some(
                            "Bearer token. Pass the token issued by the server via \
                             `Authorization: Bearer <token>`. \
                             Configure it with `--auth-token` / `ASSISTANT_WEB_TOKEN`.",
                        ))
                        .build(),
                ),
            );
        }
    }
}

/// API error response body returned for 4xx and 5xx errors.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ApiErrorResponse {
    /// Numeric error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
}

/// Assembled OpenAPI document for the Assistant web-UI.
///
/// Covers the machine-readable APIs:
/// - **agent-card** — A2A agent discovery
/// - **messages** — Send messages (unary + streaming)
/// - **tasks** — Task lifecycle management
/// - **push-notifications** — Push notification configuration
#[allow(dead_code)] // schema types are used only by the macro
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Assistant API",
        version = env!("CARGO_PKG_VERSION"),
        description = "AI assistant API — A2A protocol (Agent-to-Agent), chat, and workflow management.\n\n\
                       **Authentication**: protected endpoints require `Authorization: Bearer <token>`.\n\
                       The token is set via `--auth-token` / `ASSISTANT_WEB_TOKEN` on the server.",
        license(name = "MIT", identifier = "MIT"),
    ),
    modifiers(&BearerTokenSecurityAddon),
    paths(
        crate::api::get_capabilities,
        crate::api::list_conversations,
        crate::api::create_conversation,
        crate::api::get_conversation,
        crate::api::delete_conversation,
        crate::api::update_conversation,
        crate::api::send_message,
        crate::api::send_voice_message,
        crate::api::stream_run_events,
        crate::api::get_message_audio,
        crate::api::get_audio,
        crate::api::upload_attachment,
        crate::api::serve_attachment,
        crate::api::personas::list_personas,
        crate::api::personas::create_persona,
        crate::api::personas::set_active_persona,
        crate::api::personas::get_persona,
        crate::api::personas::get_persona_file,
        crate::api::personas::put_persona_file,
        crate::api::personas::get_skill_access,
        crate::api::personas::patch_skill_access_mode,
        crate::api::personas::add_skill_access,
        crate::api::personas::delete_skill_access,
        crate::api::skills::list_persona_skills,
        crate::api::skills::list_skills,
        crate::api::skills::create_skill,
        crate::api::skills::get_skill,
        crate::api::skills::update_skill,
        crate::api::skills::delete_skill,
        crate::api::traces::list_traces,
        crate::api::traces::get_trace,
        crate::api::logs::list_logs,
        crate::api::webhooks::list_webhooks,
        crate::api::webhooks::create_webhook,
        crate::api::webhooks::get_webhook,
        crate::api::webhooks::update_webhook,
        crate::api::webhooks::delete_webhook,
        crate::api::webhooks::toggle_webhook,
        crate::api::webhooks::rotate_secret,
        crate::api::webhooks::verify_webhook,
        crate::api::agents::list_agents,
        crate::api::agents::register_agent,
        crate::api::agents::get_agent,
        crate::api::agents::update_agent,
        crate::api::agents::delete_agent,
        crate::api::agents::set_default_agent,
        crate::api::analytics::get_analytics,
        crate::api::workflows::list_workflows,
        crate::api::workflows::create_workflow,
        crate::api::workflows::get_workflow,
        crate::api::workflows::update_workflow,
        crate::api::workflows::delete_workflow,
        crate::api::workflows::activate_workflow,
        crate::api::workflows::deactivate_workflow,
        crate::api::workflows::test_run_workflow,
        crate::api::workflows::get_workflow_webhook_secrets,
        crate::api::workflows::list_workflow_runs,
        crate::api::workflows::get_workflow_run,
        handlers::get_agent_card_well_known,
        handlers::get_extended_agent_card,
        handlers::send_message,
        handlers::send_message_streaming,
        handlers::list_tasks,
        handlers::get_task,
        handlers::cancel_task,
        handlers::subscribe_to_task,
        handlers::list_push_notification_configs,
        handlers::create_push_notification_config,
        handlers::get_push_notification_config,
        handlers::delete_push_notification_config,
        crate::api::push::vapid_public_key,
        crate::api::push::subscribe,
        crate::api::push::unsubscribe,
    ),
    components(
        schemas(
            // A2A agent card types
            AgentCard,
            AgentCapabilities,
            AgentCardSignature,
            AgentExtension,
            AgentInterface,
            AgentProvider,
            AgentSkill,
            // A2A core types
            Task,
            TaskStatus,
            TaskState,
            Message,
            Role,
            Part,
            Artifact,
            TaskStatusUpdateEvent,
            TaskArtifactUpdateEvent,
            PushNotificationConfig,
            AuthenticationInfo,
            TaskPushNotificationConfig,
            SendMessageConfiguration,
            StringList,
            // A2A request types
            SendMessageRequest,
            CancelTaskRequest,
            CreateTaskPushNotificationConfigRequest,
            // A2A response types
            SendMessageResponse,
            StreamResponse,
            ListTasksResponse,
            ListTaskPushNotificationConfigsResponse,
            // A2A security descriptor types (used inside AgentCard)
            A2ASecurityScheme,
            SecurityRequirement,
            ApiKeySecurityScheme,
            HttpAuthSecurityScheme,
            OAuth2SecurityScheme,
            OpenIdConnectSecurityScheme,
            MutualTlsSecurityScheme,
            OAuthFlows,
            AuthorizationCodeOAuthFlow,
            ClientCredentialsOAuthFlow,
            ImplicitOAuthFlow,
            PasswordOAuthFlow,
            DeviceCodeOAuthFlow,
            // Capabilities
            ServerCapabilities,
            // Conversation API types
            AttachmentMetaResponse,
            ConversationSummary,
            StreamRunEventsQuery,
            ConversationDetail,
            MessageSummary,
            ToolCallSummary,
            CreateConversationRequest,
            UpdateConversationRequest,
            ApiSendMessageRequest,
            // Flutter-facing API types
            PersonaSummary,
            SetActivePersonaRequest,
            PersonaDetail,
            PersonaFileSlot,
            PersonaFileContent,
            PersonaSkillAccess,
            CreatePersonaRequest,
            WritePersonaFileRequest,
            SetSkillAccessModeRequest,
            AddSkillAccessRequest,
            SkillEntryResponse,
            SkillDetail,
            CreateSkillRequest,
            UpdateSkillRequest,
            TraceSummaryResponse,
            SpanEntryResponse,
            TraceDetailResponse,
            LogEntryResponse,
            // Webhook API types
            WebhookResponse,
            CreateWebhookRequest,
            UpdateWebhookRequest,
            RotateSecretResponse,
            VerifyWebhookResponse,
            // Agent API types
            AgentSummary,
            AgentDetail,
            RegisterAgentRequest,
            UpdateAgentRequest,
            // Analytics API types
            AnalyticsSummaryResponse,
            ModelUsageResponse,
            ToolUsageResponse,
            TimeSeriesResponse,
            // Workflow API types
            WorkflowSummary,
            WorkflowDetail,
            WorkflowUpsertRequest,
            WorkflowRunSummary,
            WorkflowRunStep,
            WorkflowRunDetail,
            WorkflowRunPreview,
            WorkflowWebhookSecrets,
            // Web Push types
            VapidKeyResponse,
            SubscribeRequest,
            UnsubscribeRequest,
            // Common error response
            ApiErrorResponse,
        )
    ),
    tags(
        (name = "capabilities",
         description = "Server capabilities — query which optional features (voice, TTS) are enabled"),
        (name = "conversations",
         description = "Conversation management — list, create, fetch, update, delete, and send messages"),
        (name = "agent-card",
         description = "Agent discovery — retrieve the A2A agent manifest"),
        (name = "messages",
         description = "Message operations — send messages to the agent (unary or streaming)"),
        (name = "tasks",
         description = "Task lifecycle — list, retrieve, and cancel tasks"),
        (name = "push-notifications",
         description = "Push notification configuration — manage webhook callbacks for task updates"),
        (name = "personas",
         description = "Persona management — list, create, get detail, manage files and skill access"),
        (name = "skills",
         description = "Skill management — list, create, update, and delete skills"),
        (name = "webhooks",
         description = "Webhook management — create, configure, verify, and rotate secrets"),
        (name = "agents",
         description = "A2A agent registry — register and manage remote agents"),
        (name = "analytics",
         description = "Usage analytics — token consumption, model usage, and tool metrics"),
        (name = "workflows",
         description = "Workflow management — create, update, activate, and run workflows"),
        (name = "attachments",
         description = "Image attachments — upload, serve, and resize images linked to messages"),
        (name = "web-push",
         description = "Web Push — VAPID key retrieval and push subscription management for PWA notifications"),
    ),
    servers(
        (url = "/", description = "Local assistant server")
    )
)]
pub struct ApiDoc;
