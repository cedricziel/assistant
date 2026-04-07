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
import 'package:assistant_api/src/model/analytics_query_params.dart';
import 'package:assistant_api/src/model/analytics_summary_response.dart';
import 'package:assistant_api/src/model/api_error_response.dart';
import 'package:assistant_api/src/model/api_key_security_scheme.dart';
import 'package:assistant_api/src/model/api_send_message_request.dart';
import 'package:assistant_api/src/model/artifact.dart';
import 'package:assistant_api/src/model/authentication_info.dart';
import 'package:assistant_api/src/model/authorization_code_o_auth_flow.dart';
import 'package:assistant_api/src/model/cancel_task_request.dart';
import 'package:assistant_api/src/model/client_credentials_o_auth_flow.dart';
import 'package:assistant_api/src/model/conversation_detail.dart';
import 'package:assistant_api/src/model/conversation_summary.dart';
import 'package:assistant_api/src/model/create_conversation_request.dart';
import 'package:assistant_api/src/model/create_persona_request.dart';
import 'package:assistant_api/src/model/create_skill_request.dart';
import 'package:assistant_api/src/model/create_task_push_notification_config_request.dart';
import 'package:assistant_api/src/model/create_webhook_request.dart';
import 'package:assistant_api/src/model/device_code_o_auth_flow.dart';
import 'package:assistant_api/src/model/get_extended_agent_card_request.dart';
import 'package:assistant_api/src/model/get_task_push_notification_config_request.dart';
import 'package:assistant_api/src/model/get_task_request.dart';
import 'package:assistant_api/src/model/http_auth_security_scheme.dart';
import 'package:assistant_api/src/model/implicit_o_auth_flow.dart';
import 'package:assistant_api/src/model/list_task_push_notification_configs_request.dart';
import 'package:assistant_api/src/model/list_task_push_notification_configs_response.dart';
import 'package:assistant_api/src/model/list_tasks_request.dart';
import 'package:assistant_api/src/model/list_tasks_response.dart';
import 'package:assistant_api/src/model/log_entry_response.dart';
import 'package:assistant_api/src/model/message.dart';
import 'package:assistant_api/src/model/message_summary.dart';
import 'package:assistant_api/src/model/model_part.dart';
import 'package:assistant_api/src/model/model_usage_response.dart';
import 'package:assistant_api/src/model/mutual_tls_security_scheme.dart';
import 'package:assistant_api/src/model/o_auth2_security_scheme.dart';
import 'package:assistant_api/src/model/o_auth_flows.dart';
import 'package:assistant_api/src/model/open_id_connect_security_scheme.dart';
import 'package:assistant_api/src/model/password_o_auth_flow.dart';
import 'package:assistant_api/src/model/persona_detail.dart';
import 'package:assistant_api/src/model/persona_file_content.dart';
import 'package:assistant_api/src/model/persona_file_slot.dart';
import 'package:assistant_api/src/model/persona_skill_access.dart';
import 'package:assistant_api/src/model/persona_summary.dart';
import 'package:assistant_api/src/model/push_notification_config.dart';
import 'package:assistant_api/src/model/register_agent_request.dart';
import 'package:assistant_api/src/model/role.dart';
import 'package:assistant_api/src/model/rotate_secret_response.dart';
import 'package:assistant_api/src/model/security_requirement.dart';
import 'package:assistant_api/src/model/security_scheme.dart';
import 'package:assistant_api/src/model/send_message_configuration.dart';
import 'package:assistant_api/src/model/send_message_request.dart';
import 'package:assistant_api/src/model/send_message_response.dart';
import 'package:assistant_api/src/model/set_active_persona_request.dart';
import 'package:assistant_api/src/model/set_skill_access_mode_request.dart';
import 'package:assistant_api/src/model/skill_detail.dart';
import 'package:assistant_api/src/model/skill_entry_response.dart';
import 'package:assistant_api/src/model/span_entry_response.dart';
import 'package:assistant_api/src/model/stream_response.dart';
import 'package:assistant_api/src/model/string_list.dart';
import 'package:assistant_api/src/model/subscribe_to_task_request.dart';
import 'package:assistant_api/src/model/task.dart';
import 'package:assistant_api/src/model/task_artifact_update_event.dart';
import 'package:assistant_api/src/model/task_push_notification_config.dart';
import 'package:assistant_api/src/model/task_state.dart';
import 'package:assistant_api/src/model/task_status.dart';
import 'package:assistant_api/src/model/task_status_update_event.dart';
import 'package:assistant_api/src/model/time_series_response.dart';
import 'package:assistant_api/src/model/tool_usage_response.dart';
import 'package:assistant_api/src/model/trace_detail_response.dart';
import 'package:assistant_api/src/model/trace_summary_response.dart';
import 'package:assistant_api/src/model/update_agent_request.dart';
import 'package:assistant_api/src/model/update_conversation_request.dart';
import 'package:assistant_api/src/model/update_skill_request.dart';
import 'package:assistant_api/src/model/update_webhook_request.dart';
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
import 'package:assistant_api/src/model/write_persona_file_request.dart';

part 'serializers.g.dart';

@SerializersFor([
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
  AnalyticsQueryParams,
  AnalyticsSummaryResponse,
  ApiErrorResponse,
  ApiKeySecurityScheme,
  ApiSendMessageRequest,
  Artifact,
  AuthenticationInfo,
  AuthorizationCodeOAuthFlow,
  CancelTaskRequest,
  ClientCredentialsOAuthFlow,
  ConversationDetail,
  ConversationSummary,
  CreateConversationRequest,
  CreatePersonaRequest,
  CreateSkillRequest,
  CreateTaskPushNotificationConfigRequest,
  CreateWebhookRequest,
  DeviceCodeOAuthFlow,
  GetExtendedAgentCardRequest,
  GetTaskPushNotificationConfigRequest,
  GetTaskRequest,
  HttpAuthSecurityScheme,
  ImplicitOAuthFlow,
  ListTaskPushNotificationConfigsRequest,
  ListTaskPushNotificationConfigsResponse,
  ListTasksRequest,
  ListTasksResponse,
  LogEntryResponse,
  Message,
  MessageSummary,
  ModelPart,
  ModelUsageResponse,
  MutualTlsSecurityScheme,
  OAuth2SecurityScheme,
  OAuthFlows,
  OpenIdConnectSecurityScheme,
  PasswordOAuthFlow,
  PersonaDetail,
  PersonaFileContent,
  PersonaFileSlot,
  PersonaSkillAccess,
  PersonaSummary,
  PushNotificationConfig,
  RegisterAgentRequest,
  Role,
  RotateSecretResponse,
  SecurityRequirement,
  SecurityScheme,
  SendMessageConfiguration,
  SendMessageRequest,
  SendMessageResponse,
  SetActivePersonaRequest,
  SetSkillAccessModeRequest,
  SkillDetail,
  SkillEntryResponse,
  SpanEntryResponse,
  StreamResponse,
  StringList,
  SubscribeToTaskRequest,
  Task,
  TaskArtifactUpdateEvent,
  TaskPushNotificationConfig,
  TaskState,
  TaskStatus,
  TaskStatusUpdateEvent,
  TimeSeriesResponse,
  ToolUsageResponse,
  TraceDetailResponse,
  TraceSummaryResponse,
  UpdateAgentRequest,
  UpdateConversationRequest,
  UpdateSkillRequest,
  UpdateWebhookRequest,
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
  WritePersonaFileRequest,
])
Serializers serializers = (_$serializers.toBuilder()
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(ConversationSummary)]),
        () => ListBuilder<ConversationSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(WorkflowSummary)]),
        () => ListBuilder<WorkflowSummary>(),
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
        const FullType(BuiltList, [FullType(LogEntryResponse)]),
        () => ListBuilder<LogEntryResponse>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(WorkflowRunSummary)]),
        () => ListBuilder<WorkflowRunSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(AgentSummary)]),
        () => ListBuilder<AgentSummary>(),
      )
      ..addBuilderFactory(
        const FullType(BuiltList, [FullType(SkillDetail)]),
        () => ListBuilder<SkillDetail>(),
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
      ..add(Iso8601DateTimeSerializer())
    ).build();

Serializers standardSerializers =
    (serializers.toBuilder()..addPlugin(StandardJsonPlugin())).build();
