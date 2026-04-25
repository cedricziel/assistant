//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_import

import 'package:one_of_serializer/any_of_serializer.dart';
import 'package:one_of_serializer/one_of_serializer.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:built_value/standard_json_plugin.dart';
import 'package:built_value/iso_8601_date_time_serializer.dart';
import 'package:assistant_api/src/date_serializer.dart';
import 'package:assistant_api/src/model/date.dart';

import 'package:assistant_api/src/model/add_member_request.dart';
import 'package:assistant_api/src/model/add_skill_access_request.dart';
import 'package:assistant_api/src/model/agent_capabilities.dart';
import 'package:assistant_api/src/model/agent_card.dart';
import 'package:assistant_api/src/model/agent_card_signature.dart';
import 'package:assistant_api/src/model/agent_detail.dart';
import 'package:assistant_api/src/model/agent_extension.dart';
import 'package:assistant_api/src/model/agent_interface.dart';
import 'package:assistant_api/src/model/agent_provider.dart';
import 'package:assistant_api/src/model/agent_skill.dart';
import 'package:assistant_api/src/model/agent_summary.dart';
import 'package:assistant_api/src/model/analytics_summary_response.dart';
import 'package:assistant_api/src/model/api_error_response.dart';
import 'package:assistant_api/src/model/api_key_security_scheme.dart';
import 'package:assistant_api/src/model/api_key_summary.dart';
import 'package:assistant_api/src/model/api_send_message_request.dart';
import 'package:assistant_api/src/model/artifact.dart';
import 'package:assistant_api/src/model/attachment_meta_response.dart';
import 'package:assistant_api/src/model/authentication_info.dart';
import 'package:assistant_api/src/model/authorization_code_o_auth_flow.dart';
import 'package:assistant_api/src/model/binding_response.dart';
import 'package:assistant_api/src/model/cancel_task_request.dart';
import 'package:assistant_api/src/model/catalog_item_response.dart';
import 'package:assistant_api/src/model/client_credentials_o_auth_flow.dart';
import 'package:assistant_api/src/model/client_info_schema.dart';
import 'package:assistant_api/src/model/client_registration_schema.dart';
import 'package:assistant_api/src/model/command_arg_response.dart';
import 'package:assistant_api/src/model/command_def_response.dart';
import 'package:assistant_api/src/model/command_event_response.dart';
import 'package:assistant_api/src/model/conversation_detail.dart';
import 'package:assistant_api/src/model/conversation_summary.dart';
import 'package:assistant_api/src/model/create_api_key_request.dart';
import 'package:assistant_api/src/model/create_api_key_response.dart';
import 'package:assistant_api/src/model/create_binding_request.dart';
import 'package:assistant_api/src/model/create_conversation_request.dart';
import 'package:assistant_api/src/model/create_from_template_request.dart';
import 'package:assistant_api/src/model/create_interface_instance_request.dart';
import 'package:assistant_api/src/model/create_org_request.dart';
import 'package:assistant_api/src/model/create_persona_request.dart';
import 'package:assistant_api/src/model/create_skill_request.dart';
import 'package:assistant_api/src/model/create_space_request.dart';
import 'package:assistant_api/src/model/create_subscription_request.dart';
import 'package:assistant_api/src/model/create_task_push_notification_config_request.dart';
import 'package:assistant_api/src/model/create_user_request.dart';
import 'package:assistant_api/src/model/create_webhook_request.dart';
import 'package:assistant_api/src/model/device_code_o_auth_flow.dart';
import 'package:assistant_api/src/model/device_code_response_schema.dart';
import 'package:assistant_api/src/model/execute_command_request.dart';
import 'package:assistant_api/src/model/http_auth_security_scheme.dart';
import 'package:assistant_api/src/model/implicit_o_auth_flow.dart';
import 'package:assistant_api/src/model/interface_instance_response.dart';
import 'package:assistant_api/src/model/list_task_push_notification_configs_response.dart';
import 'package:assistant_api/src/model/list_tasks_response.dart';
import 'package:assistant_api/src/model/log_entry_response.dart';
import 'package:assistant_api/src/model/member_entry.dart';
import 'package:assistant_api/src/model/message.dart';
import 'package:assistant_api/src/model/message_summary.dart';
import 'package:assistant_api/src/model/model_part.dart';
import 'package:assistant_api/src/model/model_usage_response.dart';
import 'package:assistant_api/src/model/mutual_tls_security_scheme.dart';
import 'package:assistant_api/src/model/o_auth2_security_scheme.dart';
import 'package:assistant_api/src/model/o_auth_error_response.dart';
import 'package:assistant_api/src/model/o_auth_flows.dart';
import 'package:assistant_api/src/model/onboarding_status_response.dart';
import 'package:assistant_api/src/model/open_id_connect_security_scheme.dart';
import 'package:assistant_api/src/model/org_detail.dart';
import 'package:assistant_api/src/model/org_summary.dart';
import 'package:assistant_api/src/model/password_o_auth_flow.dart';
import 'package:assistant_api/src/model/persona_detail.dart';
import 'package:assistant_api/src/model/persona_file_content.dart';
import 'package:assistant_api/src/model/persona_file_slot.dart';
import 'package:assistant_api/src/model/persona_from_template_response.dart';
import 'package:assistant_api/src/model/persona_skill_access.dart';
import 'package:assistant_api/src/model/persona_summary.dart';
import 'package:assistant_api/src/model/publish_catalog_item_request.dart';
import 'package:assistant_api/src/model/push_notification_config.dart';
import 'package:assistant_api/src/model/quick_message_request.dart';
import 'package:assistant_api/src/model/quick_message_response.dart';
import 'package:assistant_api/src/model/register_agent_request.dart';
import 'package:assistant_api/src/model/role.dart';
import 'package:assistant_api/src/model/rotate_secret_response.dart';
import 'package:assistant_api/src/model/security_requirement.dart';
import 'package:assistant_api/src/model/security_scheme.dart';
import 'package:assistant_api/src/model/send_message_configuration.dart';
import 'package:assistant_api/src/model/send_message_request.dart';
import 'package:assistant_api/src/model/send_message_response.dart';
import 'package:assistant_api/src/model/server_capabilities.dart';
import 'package:assistant_api/src/model/server_metadata.dart';
import 'package:assistant_api/src/model/set_active_persona_request.dart';
import 'package:assistant_api/src/model/set_skill_access_mode_request.dart';
import 'package:assistant_api/src/model/skill_detail.dart';
import 'package:assistant_api/src/model/skill_entry_response.dart';
import 'package:assistant_api/src/model/space_detail.dart';
import 'package:assistant_api/src/model/space_summary.dart';
import 'package:assistant_api/src/model/span_entry_response.dart';
import 'package:assistant_api/src/model/sse_status_event.dart';
import 'package:assistant_api/src/model/sse_subagent_completed_event.dart';
import 'package:assistant_api/src/model/sse_subagent_started_event.dart';
import 'package:assistant_api/src/model/sse_subagent_status_event.dart';
import 'package:assistant_api/src/model/sse_subagent_thinking_event.dart';
import 'package:assistant_api/src/model/sse_subagent_token_event.dart';
import 'package:assistant_api/src/model/sse_subagent_tool_result_event.dart';
import 'package:assistant_api/src/model/sse_thinking_event.dart';
import 'package:assistant_api/src/model/sse_token_event.dart';
import 'package:assistant_api/src/model/sse_tool_result_event.dart';
import 'package:assistant_api/src/model/stream_response.dart';
import 'package:assistant_api/src/model/stream_run_events_query.dart';
import 'package:assistant_api/src/model/string_list.dart';
import 'package:assistant_api/src/model/subscribe_request.dart';
import 'package:assistant_api/src/model/subscription_response.dart';
import 'package:assistant_api/src/model/task.dart';
import 'package:assistant_api/src/model/task_artifact_update_event.dart';
import 'package:assistant_api/src/model/task_push_notification_config.dart';
import 'package:assistant_api/src/model/task_state.dart';
import 'package:assistant_api/src/model/task_status.dart';
import 'package:assistant_api/src/model/task_status_update_event.dart';
import 'package:assistant_api/src/model/template_response.dart';
import 'package:assistant_api/src/model/time_series_response.dart';
import 'package:assistant_api/src/model/token_response.dart';
import 'package:assistant_api/src/model/tool_call_summary.dart';
import 'package:assistant_api/src/model/tool_usage_response.dart';
import 'package:assistant_api/src/model/trace_detail_response.dart';
import 'package:assistant_api/src/model/trace_summary_response.dart';
import 'package:assistant_api/src/model/unsubscribe_request.dart';
import 'package:assistant_api/src/model/update_agent_request.dart';
import 'package:assistant_api/src/model/update_conversation_request.dart';
import 'package:assistant_api/src/model/update_member_request.dart';
import 'package:assistant_api/src/model/update_org_request.dart';
import 'package:assistant_api/src/model/update_skill_request.dart';
import 'package:assistant_api/src/model/update_space_request.dart';
import 'package:assistant_api/src/model/update_user_request.dart';
import 'package:assistant_api/src/model/update_webhook_request.dart';
import 'package:assistant_api/src/model/user_detail.dart';
import 'package:assistant_api/src/model/user_summary.dart';
import 'package:assistant_api/src/model/vapid_key_response.dart';
import 'package:assistant_api/src/model/verify_webhook_response.dart';
import 'package:assistant_api/src/model/webhook_response.dart';
import 'package:assistant_api/src/model/workflow_detail.dart';
import 'package:assistant_api/src/model/workflow_run_detail.dart';
import 'package:assistant_api/src/model/workflow_run_preview.dart';
import 'package:assistant_api/src/model/workflow_run_step.dart';
import 'package:assistant_api/src/model/workflow_run_summary.dart';
import 'package:assistant_api/src/model/workflow_summary.dart';
import 'package:assistant_api/src/model/workflow_upsert_request.dart';
import 'package:assistant_api/src/model/workflow_webhook_secrets.dart';
import 'package:assistant_api/src/model/workflow_webhook_trigger_accepted.dart';
import 'package:assistant_api/src/model/write_persona_file_request.dart';

part 'serializers.g.dart';

@SerializersFor([
  AddMemberRequest,
  AddSkillAccessRequest,
  AgentCapabilities,
  AgentCard,
  AgentCardSignature,
  AgentDetail,
  AgentExtension,
  AgentInterface,
  AgentProvider,
  AgentSkill,
  AgentSummary,
  AnalyticsSummaryResponse,
  ApiErrorResponse,
  ApiKeySecurityScheme,
  ApiKeySummary,
  ApiSendMessageRequest,
  Artifact,
  AttachmentMetaResponse,
  AuthenticationInfo,
  AuthorizationCodeOAuthFlow,
  BindingResponse,
  CancelTaskRequest,
  CatalogItemResponse,
  ClientCredentialsOAuthFlow,
  ClientInfoSchema,
  ClientRegistrationSchema,
  CommandArgResponse,
  CommandDefResponse,
  CommandEventResponse,
  ConversationDetail,
  ConversationSummary,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CreateBindingRequest,
  CreateConversationRequest,
  CreateFromTemplateRequest,
  CreateInterfaceInstanceRequest,
  CreateOrgRequest,
  CreatePersonaRequest,
  CreateSkillRequest,
  CreateSpaceRequest,
  CreateSubscriptionRequest,
  CreateTaskPushNotificationConfigRequest,
  CreateUserRequest,
  CreateWebhookRequest,
  DeviceCodeOAuthFlow,
  DeviceCodeResponseSchema,
  ExecuteCommandRequest,
  HttpAuthSecurityScheme,
  ImplicitOAuthFlow,
  InterfaceInstanceResponse,
  ListTaskPushNotificationConfigsResponse,
  ListTasksResponse,
  LogEntryResponse,
  MemberEntry,
  Message,
  MessageSummary,
  ModelPart,
  ModelUsageResponse,
  MutualTlsSecurityScheme,
  OAuth2SecurityScheme,
  OAuthErrorResponse,
  OAuthFlows,
  OnboardingStatusResponse,
  OpenIdConnectSecurityScheme,
  OrgDetail,
  OrgSummary,
  PasswordOAuthFlow,
  PersonaDetail,
  PersonaFileContent,
  PersonaFileSlot,
  PersonaFromTemplateResponse,
  PersonaSkillAccess,
  PersonaSummary,
  PublishCatalogItemRequest,
  PushNotificationConfig,
  QuickMessageRequest,
  QuickMessageResponse,
  RegisterAgentRequest,
  Role,
  RotateSecretResponse,
  SecurityRequirement,
  SecurityScheme,
  SendMessageConfiguration,
  SendMessageRequest,
  SendMessageResponse,
  ServerCapabilities,
  ServerMetadata,
  SetActivePersonaRequest,
  SetSkillAccessModeRequest,
  SkillDetail,
  SkillEntryResponse,
  SpaceDetail,
  SpaceSummary,
  SpanEntryResponse,
  SseStatusEvent,
  SseSubagentCompletedEvent,
  SseSubagentStartedEvent,
  SseSubagentStatusEvent,
  SseSubagentThinkingEvent,
  SseSubagentTokenEvent,
  SseSubagentToolResultEvent,
  SseThinkingEvent,
  SseTokenEvent,
  SseToolResultEvent,
  StreamResponse,
  StreamRunEventsQuery,
  StringList,
  SubscribeRequest,
  SubscriptionResponse,
  Task,
  TaskArtifactUpdateEvent,
  TaskPushNotificationConfig,
  TaskState,
  TaskStatus,
  TaskStatusUpdateEvent,
  TemplateResponse,
  TimeSeriesResponse,
  TokenResponse,
  ToolCallSummary,
  ToolUsageResponse,
  TraceDetailResponse,
  TraceSummaryResponse,
  UnsubscribeRequest,
  UpdateAgentRequest,
  UpdateConversationRequest,
  UpdateMemberRequest,
  UpdateOrgRequest,
  UpdateSkillRequest,
  UpdateSpaceRequest,
  UpdateUserRequest,
  UpdateWebhookRequest,
  UserDetail,
  UserSummary,
  VapidKeyResponse,
  VerifyWebhookResponse,
  WebhookResponse,
  WorkflowDetail,
  WorkflowRunDetail,
  WorkflowRunPreview,
  WorkflowRunStep,
  WorkflowRunSummary,
  WorkflowSummary,
  WorkflowUpsertRequest,
  WorkflowWebhookSecrets,
  WorkflowWebhookTriggerAccepted,
  WritePersonaFileRequest,
])
Serializers serializers = (_$serializers.toBuilder()
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ConversationSummary)]),
        () => ListBuilder<ConversationSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(CommandEventResponse)]),
        () => ListBuilder<CommandEventResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ApiKeySummary)]),
        () => ListBuilder<ApiKeySummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(OrgSummary)]),
        () => ListBuilder<OrgSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(WorkflowSummary)]),
        () => ListBuilder<WorkflowSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(CatalogItemResponse)]),
        () => ListBuilder<CatalogItemResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(MemberEntry)]),
        () => ListBuilder<MemberEntry>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(InterfaceInstanceResponse)]),
        () => ListBuilder<InterfaceInstanceResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(LogEntryResponse)]),
        () => ListBuilder<LogEntryResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(TemplateResponse)]),
        () => ListBuilder<TemplateResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SkillDetail)]),
        () => ListBuilder<SkillDetail>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(BindingResponse)]),
        () => ListBuilder<BindingResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(PersonaSummary)]),
        () => ListBuilder<PersonaSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(TraceSummaryResponse)]),
        () => ListBuilder<TraceSummaryResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SubscriptionResponse)]),
        () => ListBuilder<SubscriptionResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(int)]),
        () => ListBuilder<int>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(CommandDefResponse)]),
        () => ListBuilder<CommandDefResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(WorkflowRunSummary)]),
        () => ListBuilder<WorkflowRunSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(UserSummary)]),
        () => ListBuilder<UserSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SpaceSummary)]),
        () => ListBuilder<SpaceSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(AgentSummary)]),
        () => ListBuilder<AgentSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(WebhookResponse)]),
        () => ListBuilder<WebhookResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SkillEntryResponse)]),
        () => ListBuilder<SkillEntryResponse>(),
      )
      ..add(const OneOfSerializer())
      ..add(const AnyOfSerializer())
      ..add(const DateSerializer())
      ..add(Iso8601DateTimeSerializer()))
    .build();

Serializers standardSerializers =
    (serializers.toBuilder()..addPlugin(StandardJsonPlugin())).build();
