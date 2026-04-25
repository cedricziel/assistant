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
    Modify, OpenApi,
    openapi::security::{
        AuthorizationCode, ClientCredentials, Flow, HttpAuthScheme, HttpBuilder, OAuth2, Scopes,
        SecurityScheme,
    },
};

use crate::a2a::handlers;
use crate::api::push::{SubscribeRequest, UnsubscribeRequest, VapidKeyResponse};
use crate::api::{
    AttachmentMetaResponse, ConversationDetail, ConversationSummary, CreateConversationRequest,
    MessageSummary, QuickMessageRequest, QuickMessageResponse,
    SendMessageRequest as ApiSendMessageRequest, ServerCapabilities, SseStatusEvent,
    SseSubagentCompletedEvent, SseSubagentStartedEvent, SseSubagentStatusEvent,
    SseSubagentThinkingEvent, SseSubagentTokenEvent, SseSubagentToolResultEvent, SseThinkingEvent,
    SseTokenEvent, SseToolResultEvent, StreamRunEventsQuery, ToolCallSummary,
    UpdateConversationRequest,
    agents::{AgentDetail, AgentSummary, RegisterAgentRequest, UpdateAgentRequest},
    analytics::{
        AnalyticsSummaryResponse, ModelUsageResponse, TimeSeriesResponse, ToolUsageResponse,
    },
    api_keys::{ApiKeySummary, CreateApiKeyRequest, CreateApiKeyResponse},
    bindings::{BindingResponse, CreateBindingRequest},
    catalog::{
        CatalogItemResponse, CreateSubscriptionRequest, PublishCatalogItemRequest,
        SubscriptionResponse,
    },
    commands::{
        CommandArgResponse, CommandDefResponse, CommandEventResponse, ExecuteCommandRequest,
    },
    interfaces::{CreateInterfaceInstanceRequest, InterfaceInstanceResponse},
    logs::LogEntryResponse,
    members::{AddMemberRequest, MemberEntry, UpdateMemberRequest},
    orgs::{CreateOrgRequest, OrgDetail, OrgSummary, UpdateOrgRequest},
    personas::{
        AddSkillAccessRequest, CreatePersonaRequest, PersonaDetail, PersonaFileContent,
        PersonaFileSlot, PersonaSkillAccess, PersonaSummary, SetActivePersonaRequest,
        SetSkillAccessModeRequest, WritePersonaFileRequest,
    },
    skills::{CreateSkillRequest, SkillDetail, SkillEntryResponse, UpdateSkillRequest},
    spaces::{CreateSpaceRequest, SpaceDetail, SpaceSummary, UpdateSpaceRequest},
    templates::{
        CreateFromTemplateRequest, OnboardingStatusResponse, PersonaFromTemplateResponse,
        TemplateResponse,
    },
    traces::{SpanEntryResponse, TraceDetailResponse, TraceSummaryResponse},
    users::{CreateUserRequest, UpdateUserRequest, UserDetail, UserSummary},
    webhooks::{
        CreateWebhookRequest, RotateSecretResponse, UpdateWebhookRequest, VerifyWebhookResponse,
        WebhookResponse,
    },
    workflows::{
        WorkflowDetail, WorkflowRunDetail, WorkflowRunPreview, WorkflowRunStep, WorkflowRunSummary,
        WorkflowSummary, WorkflowUpsertRequest, WorkflowWebhookSecrets,
    },
};

/// Adds the Bearer token and OAuth2 security schemes to the OpenAPI components.
struct SecuritySchemesAddon;

impl Modify for SecuritySchemesAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // HTTP Bearer — legacy token or JWT issued by any flow.
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

            // OAuth2 — authorization code + client credentials flows.
            let scopes: Scopes = [
                ("conversations:read", "Read conversations"),
                ("conversations:write", "Create and send messages"),
                ("personas:read", "Read personas"),
                ("personas:write", "Create and update personas"),
                ("skills:read", "Read skills"),
                ("skills:write", "Create and update skills"),
                ("users:read", "Read users"),
                ("users:write", "Create and update users"),
                ("spaces:read", "Read spaces"),
                ("spaces:write", "Create and update spaces"),
                ("org:manage", "Full organization management"),
                ("api_keys:read", "List API keys"),
                ("api_keys:write", "Create and revoke API keys"),
            ]
            .into_iter()
            .collect();

            components.add_security_scheme(
                "oauth2",
                SecurityScheme::OAuth2(OAuth2::with_description(
                    [
                        Flow::AuthorizationCode(AuthorizationCode::with_refresh_url(
                            "/oauth/authorize",
                            "/oauth/token",
                            scopes.clone(),
                            "/oauth/token", // refresh URL = token URL
                        )),
                        Flow::ClientCredentials(ClientCredentials::new("/oauth/token", scopes)),
                    ],
                    "OAuth2 authentication. Use authorization code flow (interactive) \
                     or client credentials flow (machine-to-machine). \
                     Tokens are issued as JWTs via the `/oauth/token` endpoint.",
                )),
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
    modifiers(&SecuritySchemesAddon),
    paths(
        crate::api::get_capabilities,
        crate::api::list_conversations,
        crate::api::stream_conversations,
        crate::api::create_conversation,
        crate::api::get_conversation,
        crate::api::delete_conversation,
        crate::api::update_conversation,
        crate::api::send_message,
        crate::api::quick_message,
        crate::api::send_voice_message,
        crate::api::stream_run_events,
        crate::api::get_message_audio,
        crate::api::get_audio,
        crate::api::upload_attachment,
        crate::api::serve_attachment,
        crate::api::commands::list_commands,
        crate::api::commands::execute_command,
        crate::api::commands::list_command_events,
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
        // OAuth2 endpoints
        crate::oauth::authorize::authorize_get,
        crate::oauth::authorize::authorize_post,
        crate::oauth::token::token,
        crate::oauth::register::register,
        crate::oauth::device::device_initiate,
        crate::oauth::device::device_verify_page,
        crate::oauth::device::device_verify_submit,
        crate::oauth::callback::callback,
        crate::oauth::revoke::revoke,
        crate::oauth::revoke::metadata,
        // Organization management
        crate::api::orgs::list_orgs,
        crate::api::orgs::create_org,
        crate::api::orgs::get_org,
        crate::api::orgs::update_org,
        // User management
        crate::api::users::list_users,
        crate::api::users::create_user,
        crate::api::users::get_user,
        crate::api::users::update_user,
        crate::api::users::delete_user,
        // Space management
        crate::api::spaces::list_spaces,
        crate::api::spaces::create_space,
        crate::api::spaces::get_space,
        crate::api::spaces::update_space,
        crate::api::spaces::delete_space,
        // Membership management
        crate::api::members::list_members,
        crate::api::members::add_member,
        crate::api::members::update_member,
        crate::api::members::remove_member,
        // API key management
        crate::api::api_keys::list_api_keys,
        crate::api::api_keys::create_api_key,
        crate::api::api_keys::delete_api_key,
        // Catalog management
        crate::api::catalog::publish_catalog_item,
        crate::api::catalog::list_catalog,
        crate::api::catalog::list_catalog_by_type,
        crate::api::catalog::delete_catalog_item,
        // Catalog subscriptions
        crate::api::catalog::create_subscription,
        crate::api::catalog::list_subscriptions,
        crate::api::catalog::delete_subscription,
        // Interface instances
        crate::api::interfaces::create_interface,
        crate::api::interfaces::list_interfaces,
        crate::api::interfaces::delete_interface,
        // Persona-interface bindings
        crate::api::bindings::create_binding,
        crate::api::bindings::list_bindings,
        crate::api::bindings::delete_binding,
        // Templates & onboarding
        crate::api::templates::list_templates,
        crate::api::templates::create_from_template,
        crate::api::templates::onboarding_status,
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
            QuickMessageRequest,
            QuickMessageResponse,
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
            // Command API types
            CommandDefResponse,
            CommandArgResponse,
            CommandEventResponse,
            ExecuteCommandRequest,
            // Web Push types
            VapidKeyResponse,
            SubscribeRequest,
            UnsubscribeRequest,
            // SSE event payload schemas
            SseTokenEvent,
            SseThinkingEvent,
            SseStatusEvent,
            SseToolResultEvent,
            SseSubagentStartedEvent,
            SseSubagentCompletedEvent,
            SseSubagentTokenEvent,
            SseSubagentThinkingEvent,
            SseSubagentToolResultEvent,
            SseSubagentStatusEvent,
            // OAuth2 types
            crate::oauth::OAuthErrorResponse,
            crate::oauth::token::TokenRequest,
            crate::oauth::token::TokenResponse,
            crate::oauth::authorize::AuthorizeForm,
            crate::oauth::device::DeviceInitiateRequest,
            crate::oauth::device::DeviceVerifyForm,
            crate::oauth::revoke::RevokeRequest,
            crate::oauth::revoke::ServerMetadata,
            crate::oauth::DeviceCodeResponseSchema,
            crate::oauth::ClientRegistrationSchema,
            crate::oauth::ClientInfoSchema,
            // Common error response
            ApiErrorResponse,
            // Organization management types
            OrgSummary,
            OrgDetail,
            CreateOrgRequest,
            UpdateOrgRequest,
            // User management types
            UserSummary,
            UserDetail,
            CreateUserRequest,
            UpdateUserRequest,
            // Space management types
            SpaceSummary,
            SpaceDetail,
            CreateSpaceRequest,
            UpdateSpaceRequest,
            // Membership management types
            MemberEntry,
            AddMemberRequest,
            UpdateMemberRequest,
            // API key management types
            ApiKeySummary,
            CreateApiKeyRequest,
            CreateApiKeyResponse,
            // Catalog management types
            CatalogItemResponse,
            PublishCatalogItemRequest,
            SubscriptionResponse,
            CreateSubscriptionRequest,
            // Interface instance types
            InterfaceInstanceResponse,
            CreateInterfaceInstanceRequest,
            // Binding types
            BindingResponse,
            CreateBindingRequest,
            // Template / onboarding types
            TemplateResponse,
            CreateFromTemplateRequest,
            PersonaFromTemplateResponse,
            OnboardingStatusResponse,
        )
    ),
    tags(
        (name = "capabilities",
         description = "Server capabilities — query which optional features (voice, TTS) are enabled"),
        (name = "commands",
         description = "Slash commands — list, execute, and view command events for conversations"),
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
        (name = "oauth",
         description = "OAuth2 — authorization, token exchange, device code flow, client registration, and server metadata"),
        (name = "orgs",
         description = "Organization management — list, create, update organizations"),
        (name = "users",
         description = "User management — list, create, update, delete users within an organization"),
        (name = "spaces",
         description = "Space management — list, create, update, delete spaces within an organization"),
        (name = "members",
         description = "Membership management — add, update, remove space members"),
        (name = "api-keys",
         description = "API key management — create, list, revoke API keys"),
        (name = "catalog",
         description = "Catalog management — publish, list, and remove shared resources; manage space subscriptions"),
        (name = "interfaces",
         description = "Interface instances — create, list, and delete per-space interface configurations"),
        (name = "bindings",
         description = "Persona-interface bindings — bind personas to interfaces within a space"),
        (name = "templates",
         description = "Persona templates — list templates, instantiate personas, check onboarding status"),
    ),
    servers(
        (url = "/", description = "Local assistant server")
    )
)]
pub struct ApiDoc;
