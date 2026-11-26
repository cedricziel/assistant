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
    openapi::{
        ObjectBuilder, RefOr, Required,
        path::{Parameter, ParameterBuilder, ParameterIn},
    },
};

use crate::a2a::handlers;
use crate::api::attachments::AttachmentMetaResponse;
use crate::api::capabilities::{ServerCapabilities, UiCapabilities};
use crate::api::conversations::{
    ConversationDetail, ConversationSearchSummary, ConversationSummary, CreateConversationRequest,
    MessageFeedbackSummary, MessageSearchMatch, MessageSummary, SearchSegment, ToolCallSummary,
    UpdateConversationRequest,
};
use crate::api::messages::{
    QuickMessageRequest, QuickMessageResponse, SendMessageRequest as ApiSendMessageRequest,
    SseStatusEvent, SseSubagentCompletedEvent, SseSubagentStartedEvent, SseSubagentStatusEvent,
    SseSubagentThinkingEvent, SseSubagentTokenEvent, SseSubagentToolResultEvent, SseThinkingEvent,
    SseTokenEvent, SseToolResultEvent, StreamRunEventsQuery,
};
use crate::api::push::{SubscribeRequest, UnsubscribeRequest, VapidKeyResponse};
use crate::api::surfaces::SurfaceSummary;
use crate::api::{
    account::{ChangePasswordRequest, UpdateCurrentUserRequest, UpdateCurrentUserResponse},
    agents::{AgentDetail, AgentSummary, RegisterAgentRequest, UpdateAgentRequest},
    api_keys::{ApiKeySummary, CreateApiKeyRequest, CreateApiKeyResponse},
    bindings::{BindingResponse, CreateBindingRequest},
    catalog::{
        CatalogItemResponse, CreateSubscriptionRequest, PublishCatalogItemRequest,
        SubscriptionResponse,
    },
    commands::{
        CommandArgResponse, CommandDefResponse, CommandEventResponse, ExecuteCommandRequest,
    },
    integrations::{
        CatalogAuthKindResponse, CatalogAuthResponse, CatalogEntryListResponse,
        CatalogEntrySummary,
        types::{
            CatalogMode, CreateIntegrationRequest, CredentialMode, CredentialResponse,
            IntegrationAuthKind, IntegrationDetail, IntegrationLayer, IntegrationListResponse,
            IntegrationPolicyResponse, IntegrationStatus, IntegrationSummary, IntegrationTrust,
            SetCredentialRequest, SetSpaceAccessRequest, SpaceAccessEntry, SpaceAccessMode,
            TestIntegrationResponse, UpdateIntegrationPolicyRequest, UpdateIntegrationRequest,
        },
    },
    interfaces::{CreateInterfaceInstanceRequest, InterfaceInstanceResponse},
    members::{AddMemberRequest, MemberEntry, UpdateMemberRequest},
    org_oidc::{OrgOidcConfigResponse, PutOrgOidcConfigRequest},
    orgs::{CreateOrgRequest, OrgDetail, OrgSummary, UpdateOrgRequest},
    personas::{
        AddSkillAccessRequest, CreatePersonaRequest, PersonaDetail, PersonaFileContent,
        PersonaFileSlot, PersonaSkillAccess, PersonaSummary, SetActivePersonaRequest,
        SetSkillAccessModeRequest, UpdatePersonaRequest, WritePersonaFileRequest,
    },
    skills::{CreateSkillRequest, SkillDetail, SkillEntryResponse, UpdateSkillRequest},
    spaces::{CreateSpaceRequest, SpaceDetail, SpaceSummary, UpdateSpaceRequest},
    templates::{
        CreateFromTemplateRequest, OnboardingStatusResponse, PersonaFromTemplateResponse,
        TemplateResponse,
    },
    tool_permission_rules::{RuleScope, ToolPermissionRuleSummary},
    users::{CreateUserRequest, UpdateUserRequest, UserDetail, UserSummary},
    webhooks::{
        CreateWebhookRequest, RotateSecretResponse, UpdateWebhookRequest, VerifyWebhookResponse,
        WebhookResponse,
    },
    workflows::{
        WorkflowDetail, WorkflowRunDetail, WorkflowRunPreview, WorkflowRunStep, WorkflowRunSummary,
        WorkflowSummary, WorkflowUpsertRequest, WorkflowWebhookSecrets,
        WorkflowWebhookTriggerAccepted,
    },
};

/// Path prefixes of the data-plane routes, which act inside one space.
const DATA_PLANE_PREFIXES: [&str; 14] = [
    "/api/conversations",
    "/api/quick-message",
    "/api/attachments",
    "/api/messages",
    "/api/audio",
    "/api/commands",
    "/api/capabilities",
    "/api/push",
    "/api/webhooks",
    "/api/workflows",
    "/api/personas",
    "/api/skills",
    "/message",
    "/tasks",
];

/// Documents the `X-Space-Id` header on every data-plane operation.
struct SpaceSelectorAddon;

impl Modify for SpaceSelectorAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        for (path, item) in openapi.paths.paths.iter_mut() {
            let is_data_plane = DATA_PLANE_PREFIXES
                .iter()
                .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")));
            if !is_data_plane {
                continue;
            }
            for operation in [
                &mut item.get,
                &mut item.put,
                &mut item.post,
                &mut item.delete,
                &mut item.patch,
            ]
            .into_iter()
            .flatten()
            {
                operation
                    .parameters
                    .get_or_insert_with(Vec::new)
                    .push(RefOr::T(space_header_parameter()));
            }
        }
    }
}

fn space_header_parameter() -> Parameter {
    ParameterBuilder::new()
        .name("X-Space-Id")
        .parameter_in(ParameterIn::Header)
        .required(Required::False)
        .description(Some(
            "Id of the space to act in (from `GET /api/orgs/{org_id}/spaces`). Optional when \
             the caller holds a role in exactly one space. Clients that cannot set headers, \
             such as the browser `EventSource`, may send the same value as the `space_id` \
             query parameter.",
        ))
        .schema(Some(
            ObjectBuilder::new().schema_type(utoipa::openapi::Type::String),
        ))
        .build()
}

/// Adds the Bearer token and OAuth2 security schemes to the OpenAPI components.
struct SecuritySchemesAddon;

impl Modify for SecuritySchemesAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // HTTP Bearer — OAuth2 access token (JWT) or API key.
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some(
                            "Bearer token. Pass an OAuth2 access token issued by the server's \
                             `/oauth/token` endpoint, or an API key (`ask_live_...`), via \
                             `Authorization: Bearer <token>`.",
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
                ("mcp:execute", "Call tools through the MCP endpoint"),
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

/// Standard error envelope returned for every 4xx/5xx response from
/// `/api/...` endpoints. Matches the project convention documented in
/// `AGENTS.md`: `{"error": "...message..."}`.
///
/// OAuth2 endpoints under `/oauth/...` use [`crate::oauth::OAuthErrorResponse`]
/// (RFC 6749 §5.2) and are intentionally exempt.
///
/// One more exception: a `401 Unauthorized` is sent by the auth middleware
/// before any handler runs, as the plain text `Unauthorized`, not as JSON.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
    /// Human-readable error message.
    pub error: String,
}

/// Legacy error envelope. Replaced by [`ErrorBody`] across all `/api` routes.
/// Removed in phase 3 of this change once no handler references it.
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
                       The token is an OAuth2 access token issued by `/oauth/token`, or an API key (`ask_live_...`).",
        license(name = "MIT", identifier = "MIT"),
    ),
    modifiers(&SecuritySchemesAddon, &SpaceSelectorAddon),
    paths(
        crate::api::capabilities::get_capabilities,
        crate::api::conversations::list_conversations,
        crate::api::conversations::stream_conversations,
        crate::api::conversations::search_conversations,
        crate::api::conversations::create_conversation,
        crate::api::conversations::get_conversation,
        crate::api::conversations::delete_conversation,
        crate::api::conversations::update_conversation,
        crate::api::messages::send_message,
        crate::api::feedback::set_message_feedback,
        crate::api::feedback::delete_message_feedback,
        crate::api::messages::quick_message,
        crate::api::messages::send_voice_message,
        crate::api::messages::stream_run_events,
        crate::api::turns::get_turn_status,
        crate::api::turns::cancel_turn,
        crate::api::interactions::respond_to_interaction,
        crate::api::mcp_apps::call_mcp_app_tool,
        crate::api::mcp_apps::read_mcp_app_resource,
        crate::api::audio::get_message_audio,
        crate::api::audio::get_audio,
        crate::api::attachments::upload_attachment,
        crate::api::attachments::serve_attachment,
        crate::api::commands::list_commands,
        crate::api::commands::execute_command,
        crate::api::commands::list_command_events,
        crate::api::personas::list_personas,
        crate::api::personas::create_persona,
        crate::api::personas::set_active_persona,
        crate::api::personas::get_persona,
        crate::api::personas::update_persona,
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
        crate::api::workflows::public_webhook_trigger,
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
        crate::api::tool_permission_rules::list_tool_permission_rules,
        crate::api::tool_permission_rules::delete_tool_permission_rule,
        // OAuth2 endpoints
        crate::oauth::authorize::authorize_get,
        crate::oauth::authorize::authorize_post,
        crate::oauth::authorize::identify_post,
        crate::oauth::token::token,
        crate::oauth::register::register,
        crate::oauth::device::device_initiate,
        crate::oauth::device::device_verify_page,
        crate::oauth::device::device_verify_submit,
        crate::oauth::callback::callback,
        crate::oauth::complete::complete,
        crate::oauth::revoke::revoke,
        crate::oauth::revoke::metadata,
        crate::mcp::protected_resource_metadata,
        // Organization management
        crate::api::orgs::list_orgs,
        crate::api::orgs::create_org,
        crate::api::orgs::get_org,
        crate::api::orgs::update_org,
        crate::api::org_oidc::get_org_oidc,
        crate::api::org_oidc::put_org_oidc,
        crate::api::org_oidc::delete_org_oidc,
        // User management
        crate::api::users::list_users,
        crate::api::users::create_user,
        crate::api::users::get_user,
        crate::api::users::update_user,
        crate::api::users::delete_user,
        // Self-service account
        crate::api::account::get_current_user,
        crate::api::account::update_current_user,
        crate::api::account::change_password,
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
        // Integrations catalog
        crate::api::integrations::list_catalog_entries,
        crate::api::integrations::get_catalog_entry,
        // Integrations
        crate::api::integrations::manage::list_integrations,
        crate::api::integrations::manage::create_integration,
        crate::api::integrations::manage::get_integration,
        crate::api::integrations::manage::update_integration,
        crate::api::integrations::manage::delete_integration,
        crate::api::integrations::policy::get_integration_policy,
        crate::api::integrations::policy::put_integration_policy,
        crate::api::integrations::connection::test_integration,
        crate::api::integrations::credentials::set_credential,
        crate::api::integrations::credentials::clear_credential,
        crate::api::integrations::spaces::put_space_access,
        crate::api::integrations::spaces::delete_space_access,
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
            UiCapabilities,
            // Conversation API types
            AttachmentMetaResponse,
            ConversationSummary,
            ConversationSearchSummary,
            MessageSearchMatch,
            SearchSegment,
            StreamRunEventsQuery,
            ConversationDetail,
            MessageSummary,
            MessageFeedbackSummary,
            ToolCallSummary,
            SurfaceSummary,
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
            UpdatePersonaRequest,
            AddSkillAccessRequest,
            SkillEntryResponse,
            SkillDetail,
            CreateSkillRequest,
            UpdateSkillRequest,
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
            // Workflow API types
            WorkflowSummary,
            WorkflowDetail,
            WorkflowUpsertRequest,
            WorkflowRunSummary,
            WorkflowRunStep,
            WorkflowRunDetail,
            WorkflowRunPreview,
            WorkflowWebhookSecrets,
            WorkflowWebhookTriggerAccepted,
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
            // Turn status types
            crate::api::turns::TurnState,
            crate::api::turns::TurnStatusResponse,
            // Turn interaction types
            crate::api::interactions::RespondToInteractionRequest,
            crate::api::mcp_apps::CallMcpAppToolRequest,
            crate::api::mcp_apps::ReadMcpAppResourceRequest,
            crate::api::mcp_apps::McpAppView,
            crate::api::mcp_apps::McpAppCspView,
            crate::api::mcp_apps::McpAppPermissionsView,
            crate::api::interactions::ToolDecision,
            crate::api::interactions::QuestionAnswer,
            crate::api::interactions::InteractionDetail,
            crate::api::interactions::InteractionStatusView,
            crate::api::interactions::InteractionOutcomeView,
            crate::api::interactions::SseInteractionRequestedEvent,
            crate::api::interactions::SseInteractionResolvedEvent,
            // OAuth2 types
            crate::oauth::OAuthErrorResponse,
            crate::oauth::token::TokenRequest,
            crate::oauth::token::TokenResponse,
            crate::oauth::authorize::AuthorizeForm,
            crate::oauth::authorize::IdentifyForm,
            crate::oauth::device::DeviceInitiateRequest,
            crate::oauth::device::DeviceVerifyForm,
            crate::oauth::revoke::RevokeRequest,
            crate::oauth::revoke::ServerMetadata,
            crate::mcp::ProtectedResourceMetadata,
            crate::oauth::complete::CompleteResponse,
            crate::oauth::DeviceCodeResponseSchema,
            crate::oauth::ClientRegistrationSchema,
            crate::oauth::ClientInfoSchema,
            // Common error response
            ErrorBody,
            crate::api::feedback::SetMessageFeedbackRequest,
            crate::api::feedback::MessageFeedbackResponse,
            crate::api::feedback::FeedbackRating,
            ApiErrorResponse,
            // Organization management types
            OrgSummary,
            OrgDetail,
            CreateOrgRequest,
            UpdateOrgRequest,
            OrgOidcConfigResponse,
            PutOrgOidcConfigRequest,
            // User management types
            UserSummary,
            UserDetail,
            CreateUserRequest,
            UpdateUserRequest,
            // Self-service account types
            UpdateCurrentUserRequest,
            UpdateCurrentUserResponse,
            ChangePasswordRequest,
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
            // Integrations catalog types
            CatalogEntrySummary,
            CatalogEntryListResponse,
            CatalogAuthResponse,
            CatalogAuthKindResponse,
            IntegrationLayer,
            IntegrationAuthKind,
            CredentialMode,
            IntegrationTrust,
            SpaceAccessMode,
            CatalogMode,
            IntegrationStatus,
            IntegrationSummary,
            IntegrationDetail,
            IntegrationListResponse,
            SpaceAccessEntry,
            SetCredentialRequest,
            CredentialResponse,
            TestIntegrationResponse,
            CreateIntegrationRequest,
            UpdateIntegrationRequest,
            IntegrationPolicyResponse,
            UpdateIntegrationPolicyRequest,
            SetSpaceAccessRequest,
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
            // Tool permission rules
            ToolPermissionRuleSummary,
            RuleScope,
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
        (name = "account",
         description = "Self-service account — view and update the caller's own profile and password"),
        (name = "spaces",
         description = "Space management — list, create, update, delete spaces within an organization"),
        (name = "members",
         description = "Membership management — add, update, remove space members"),
        (name = "api-keys",
         description = "API key management — create, list, revoke API keys"),
        (name = "catalog",
         description = "Catalog management — publish, list, and remove shared resources; manage space subscriptions"),
        (name = "integrations",
         description = "Integrations — remote MCP servers installed at the org, space or user layer, the org policy that limits them, and the cached integrations.sh catalog"),
        (name = "interfaces",
         description = "Interface instances — create, list, and delete per-space interface configurations"),
        (name = "bindings",
         description = "Persona-interface bindings — bind personas to interfaces within a space"),
        (name = "templates",
         description = "Persona templates — list templates, instantiate personas, check onboarding status"),
        (name = "tool-permission-rules",
         description = "Saved tool permission rules — list the caller's rules (or the space's, for admins) and revoke them"),
    ),
    servers(
        (url = "/", description = "Local assistant server")
    )
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use utoipa::OpenApi;

    use super::ApiDoc;

    fn spec_json() -> serde_json::Value {
        serde_json::to_value(ApiDoc::openapi()).expect("spec serialises")
    }

    /// The committed `openapi.json` is the contract the Dart client is generated
    /// from, so it must equal what the code produces.
    #[test]
    fn committed_openapi_json_matches_code() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../openapi.json");
        let committed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("openapi.json readable"))
                .expect("openapi.json is valid JSON");
        assert!(
            committed == spec_json(),
            "openapi.json is out of date with the code; run `make dump-openapi` and commit the result"
        );
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src dir readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Names of the functions annotated with `#[utoipa::path(...)]` in `source`.
    fn annotated_handlers(source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut pending = false;
        for line in source.lines() {
            let line = line.trim_start();
            if line.starts_with("#[utoipa::path(") {
                pending = true;
            } else if pending {
                let signature = line
                    .strip_prefix("pub ")
                    .unwrap_or(line)
                    .strip_prefix("async ")
                    .unwrap_or(line);
                if let Some(rest) = signature.strip_prefix("fn ") {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    names.push(name);
                    pending = false;
                }
            }
        }
        names
    }

    /// `module::handler` for every `#[utoipa::path]` in `file`.
    fn qualified_handlers(file: &Path) -> Vec<String> {
        let module = match file.file_stem().and_then(|s| s.to_str()) {
            Some("mod") => file.parent().and_then(|p| p.file_name()),
            stem => stem.map(std::ffi::OsStr::new),
        }
        .and_then(|m| m.to_str())
        .expect("module name")
        .to_owned();
        let source = std::fs::read_to_string(file).expect("source readable");
        annotated_handlers(&source)
            .into_iter()
            .map(|name| format!("{module}::{name}"))
            .collect()
    }

    /// A handler with `#[utoipa::path]` that is missing from `ApiDoc`'s
    /// `paths(...)` list silently drops out of the spec and the Dart client.
    #[test]
    fn every_annotated_handler_is_listed_in_api_doc() {
        let listed: BTreeSet<String> = include_str!("openapi.rs")
            .lines()
            .filter_map(|line| line.trim().strip_suffix(','))
            .filter(|entry| {
                entry
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
            })
            .filter_map(|entry| {
                let mut segments = entry.rsplit("::");
                let name = segments.next()?;
                Some(format!("{}::{name}", segments.next()?))
            })
            .collect();

        let mut files = Vec::new();
        rust_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let missing: Vec<String> = files
            .iter()
            .filter(|file| !file.ends_with("openapi.rs"))
            .flat_map(|file| qualified_handlers(file))
            .filter(|handler| !listed.contains(handler))
            .collect();
        assert!(
            missing.is_empty(),
            "handlers with #[utoipa::path] missing from ApiDoc paths(...) in crates/web-ui/src/openapi.rs: {missing:?}"
        );
    }

    fn space_header_operations(spec: &serde_json::Value) -> Vec<String> {
        let mut documented = Vec::new();
        for (path, item) in spec["paths"].as_object().expect("paths object") {
            for (method, op) in item.as_object().expect("path item") {
                let has_header = op["parameters"].as_array().is_some_and(|params| {
                    params
                        .iter()
                        .any(|p| p["name"] == "X-Space-Id" && p["in"] == "header")
                });
                if has_header {
                    documented.push(format!("{method} {path}"));
                }
            }
        }
        documented
    }

    /// Data-plane operations take their space from `X-Space-Id`; control-plane
    /// and public operations do not.
    #[test]
    fn data_plane_operations_document_the_space_selector() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("spec serialises");
        let documented = space_header_operations(&spec);

        for expected in [
            "get /api/conversations",
            "post /api/conversations/{id}/messages",
            "put /api/conversations/{id}/messages/{message_id}/feedback",
            "delete /api/conversations/{id}/messages/{message_id}/feedback",
            "get /api/attachments/{id}",
            "get /api/capabilities",
            "post /api/webhooks",
            "put /api/workflows/{id}",
            "post /api/push/subscribe",
        ] {
            assert!(
                documented.contains(&expected.to_string()),
                "{expected} must document X-Space-Id; documented: {documented:?}"
            );
        }
        for excluded in [
            "get /api/orgs",
            "get /api/spaces",
            "post /workflow-hooks/{org_slug}/{space_slug}/{id}/{token}",
        ] {
            assert!(
                !documented.contains(&excluded.to_string()),
                "{excluded} must not document X-Space-Id"
            );
        }
    }

    /// `tool_result` frames carry JSON objects for `arguments` and `data`, and
    /// the call id, so generated clients decode them as maps, not strings.
    #[test]
    fn sse_tool_result_event_documents_object_arguments_data_and_call_id() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("spec serialises");
        let schema = &spec["components"]["schemas"]["SseToolResultEvent"];
        let props = &schema["properties"];
        for field in ["arguments", "data"] {
            let ty = &props[field]["type"];
            let is_object =
                ty == "object" || ty.as_array().is_some_and(|t| t.contains(&"object".into()));
            assert!(
                is_object,
                "{field} must be type object, got {}",
                props[field]
            );
        }
        assert!(
            props["tool_call_id"].is_object(),
            "tool_call_id must be documented: {props}"
        );
        let required = schema["required"].as_array().expect("required list");
        assert!(
            !required.iter().any(|r| r == "tool_call_id" || r == "data"),
            "tool_call_id and data stay optional: {required:?}"
        );
        let data_doc = props["data"]["description"].as_str().unwrap_or_default();
        assert!(
            data_doc.contains("schemas/tool-results/"),
            "data must point at schemas/tool-results/: {data_doc}"
        );
    }

    /// History summaries and `tool_result` frames expose the MCP App as a
    /// typed model, so generated clients get its HTML, CSP and permissions.
    #[test]
    fn tool_results_document_the_mcp_app() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("spec serialises");
        let schemas = &spec["components"]["schemas"];
        for owner in ["ToolCallSummary", "SseToolResultEvent"] {
            let app = serde_json::to_string(&schemas[owner]["properties"]["app"]).unwrap();
            assert!(
                app.contains("#/components/schemas/McpAppView"),
                "{owner}.app must reference McpAppView: {app}"
            );
        }
        let props = &schemas["McpAppView"]["properties"];
        for field in ["server", "resource_uri", "html", "csp", "permissions"] {
            assert!(props[field].is_object(), "McpAppView.{field} is documented");
        }
    }

    /// History summaries expose tool data as an object and the call id.
    #[test]
    fn tool_call_summary_documents_object_data_and_call_id() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("spec serialises");
        let schema = &spec["components"]["schemas"]["ToolCallSummary"];
        let props = &schema["properties"];
        let ty = &props["data"]["type"];
        assert!(
            ty == "object" || ty.as_array().is_some_and(|t| t.contains(&"object".into())),
            "data must be type object, got {}",
            props["data"]
        );
        assert!(props["tool_call_id"].is_object(), "tool_call_id documented");
        let required = schema["required"].as_array().expect("required list");
        assert!(
            !required.iter().any(|r| r == "tool_call_id" || r == "data"),
            "tool_call_id and data stay optional: {required:?}"
        );
        let doc = props["data"]["description"].as_str().unwrap_or_default();
        assert!(doc.contains("schemas/tool-results/"), "data doc: {doc}");
    }

    /// Telemetry is emitted over OTLP only; the REST API must not expose
    /// local trace, log or analytics viewers.
    #[test]
    fn spec_has_no_local_telemetry_endpoints() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("spec serialises");
        let paths = spec["paths"].as_object().expect("paths object");
        for banned in ["/api/traces", "/api/logs", "/api/analytics"] {
            let offenders: Vec<_> = paths.keys().filter(|p| p.starts_with(banned)).collect();
            assert!(
                offenders.is_empty(),
                "{banned} must not be documented, found {offenders:?}"
            );
        }
        let schemas = spec["components"]["schemas"]
            .as_object()
            .expect("schemas object");
        for banned in [
            "TraceSummaryResponse",
            "TraceDetailResponse",
            "SpanEntryResponse",
            "LogEntryResponse",
            "AnalyticsSummaryResponse",
        ] {
            assert!(
                !schemas.contains_key(banned),
                "schema {banned} must be removed"
            );
        }
    }
}
