// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'serializers.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

Serializers _$serializers = (Serializers().toBuilder()
      ..add(AddSkillAccessRequest.serializer)
      ..add(AgentCapabilities.serializer)
      ..add(AgentCard.serializer)
      ..add(AgentCardSignature.serializer)
      ..add(AgentDetail.serializer)
      ..add(AgentExtension.serializer)
      ..add(AgentInterface.serializer)
      ..add(AgentProvider.serializer)
      ..add(AgentSkill.serializer)
      ..add(AgentSummary.serializer)
      ..add(AnalyticsSummaryResponse.serializer)
      ..add(ApiErrorResponse.serializer)
      ..add(ApiKeySecurityScheme.serializer)
      ..add(ApiSendMessageRequest.serializer)
      ..add(Artifact.serializer)
      ..add(AuthenticationInfo.serializer)
      ..add(AuthorizationCodeOAuthFlow.serializer)
      ..add(CancelTaskRequest.serializer)
      ..add(ClientCredentialsOAuthFlow.serializer)
      ..add(ConversationDetail.serializer)
      ..add(ConversationSummary.serializer)
      ..add(CreateConversationRequest.serializer)
      ..add(CreatePersonaRequest.serializer)
      ..add(CreateSkillRequest.serializer)
      ..add(CreateTaskPushNotificationConfigRequest.serializer)
      ..add(CreateWebhookRequest.serializer)
      ..add(DeviceCodeOAuthFlow.serializer)
      ..add(HttpAuthSecurityScheme.serializer)
      ..add(ImplicitOAuthFlow.serializer)
      ..add(ListTaskPushNotificationConfigsResponse.serializer)
      ..add(ListTasksResponse.serializer)
      ..add(LogEntryResponse.serializer)
      ..add(Message.serializer)
      ..add(MessageSummary.serializer)
      ..add(ModelPart.serializer)
      ..add(ModelUsageResponse.serializer)
      ..add(MutualTlsSecurityScheme.serializer)
      ..add(OAuth2SecurityScheme.serializer)
      ..add(OAuthFlows.serializer)
      ..add(OpenIdConnectSecurityScheme.serializer)
      ..add(PasswordOAuthFlow.serializer)
      ..add(PersonaDetail.serializer)
      ..add(PersonaFileContent.serializer)
      ..add(PersonaFileSlot.serializer)
      ..add(PersonaSkillAccess.serializer)
      ..add(PersonaSummary.serializer)
      ..add(PushNotificationConfig.serializer)
      ..add(RegisterAgentRequest.serializer)
      ..add(Role.serializer)
      ..add(RotateSecretResponse.serializer)
      ..add(SecurityRequirement.serializer)
      ..add(SecurityScheme.serializer)
      ..add(SendMessageConfiguration.serializer)
      ..add(SendMessageRequest.serializer)
      ..add(SendMessageResponse.serializer)
      ..add(ServerCapabilities.serializer)
      ..add(SetActivePersonaRequest.serializer)
      ..add(SetSkillAccessModeRequest.serializer)
      ..add(SkillDetail.serializer)
      ..add(SkillEntryResponse.serializer)
      ..add(SpanEntryResponse.serializer)
      ..add(StreamResponse.serializer)
      ..add(StringList.serializer)
      ..add(SubscribeRequest.serializer)
      ..add(Task.serializer)
      ..add(TaskArtifactUpdateEvent.serializer)
      ..add(TaskPushNotificationConfig.serializer)
      ..add(TaskState.serializer)
      ..add(TaskStatus.serializer)
      ..add(TaskStatusUpdateEvent.serializer)
      ..add(TimeSeriesResponse.serializer)
      ..add(ToolUsageResponse.serializer)
      ..add(TraceDetailResponse.serializer)
      ..add(TraceSummaryResponse.serializer)
      ..add(UnsubscribeRequest.serializer)
      ..add(UpdateAgentRequest.serializer)
      ..add(UpdateConversationRequest.serializer)
      ..add(UpdateSkillRequest.serializer)
      ..add(UpdateWebhookRequest.serializer)
      ..add(VapidKeyResponse.serializer)
      ..add(VerifyWebhookResponse.serializer)
      ..add(WebhookResponse.serializer)
      ..add(WorkflowDetail.serializer)
      ..add(WorkflowRunDetail.serializer)
      ..add(WorkflowRunPreview.serializer)
      ..add(WorkflowRunStep.serializer)
      ..add(WorkflowRunSummary.serializer)
      ..add(WorkflowSummary.serializer)
      ..add(WorkflowUpsertRequest.serializer)
      ..add(WorkflowWebhookSecrets.serializer)
      ..add(WritePersonaFileRequest.serializer)
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(AgentExtension)]),
          () => ListBuilder<AgentExtension>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(Artifact)]),
          () => ListBuilder<Artifact>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(Message)]),
          () => ListBuilder<Message>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(MessageSummary)]),
          () => ListBuilder<MessageSummary>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(ModelUsageResponse)]),
          () => ListBuilder<ModelUsageResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(TimeSeriesResponse)]),
          () => ListBuilder<TimeSeriesResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(TimeSeriesResponse)]),
          () => ListBuilder<TimeSeriesResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(ToolUsageResponse)]),
          () => ListBuilder<ToolUsageResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(PersonaFileSlot)]),
          () => ListBuilder<PersonaFileSlot>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(SpanEntryResponse)]),
          () => ListBuilder<SpanEntryResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(
              BuiltList, const [const FullType(SecurityRequirement)]),
          () => ListBuilder<SecurityRequirement>())
      ..addBuilderFactory(
          const FullType(BuiltMap,
              const [const FullType(String), const FullType(SecurityScheme)]),
          () => MapBuilder<String, SecurityScheme>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(AgentCardSignature)]),
          () => ListBuilder<AgentCardSignature>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(AgentSkill)]),
          () => ListBuilder<AgentSkill>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(AgentInterface)]),
          () => ListBuilder<AgentInterface>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(
              BuiltList, const [const FullType(SecurityRequirement)]),
          () => ListBuilder<SecurityRequirement>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(ModelPart)]),
          () => ListBuilder<ModelPart>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(ModelPart)]),
          () => ListBuilder<ModelPart>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(String)]),
          () => ListBuilder<String>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(Task)]),
          () => ListBuilder<Task>())
      ..addBuilderFactory(
          const FullType(
              BuiltList, const [const FullType(TaskPushNotificationConfig)]),
          () => ListBuilder<TaskPushNotificationConfig>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(WorkflowRunStep)]),
          () => ListBuilder<WorkflowRunStep>())
      ..addBuilderFactory(
          const FullType(
              BuiltMap, const [const FullType(String), const FullType(String)]),
          () => MapBuilder<String, String>())
      ..addBuilderFactory(
          const FullType(
              BuiltMap, const [const FullType(String), const FullType(String)]),
          () => MapBuilder<String, String>())
      ..addBuilderFactory(
          const FullType(
              BuiltMap, const [const FullType(String), const FullType(String)]),
          () => MapBuilder<String, String>())
      ..addBuilderFactory(
          const FullType(
              BuiltMap, const [const FullType(String), const FullType(String)]),
          () => MapBuilder<String, String>())
      ..addBuilderFactory(
          const FullType(
              BuiltMap, const [const FullType(String), const FullType(String)]),
          () => MapBuilder<String, String>())
      ..addBuilderFactory(
          const FullType(BuiltMap,
              const [const FullType(String), const FullType(StringList)]),
          () => MapBuilder<String, StringList>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>())
      ..addBuilderFactory(
          const FullType(BuiltMap, const [
            const FullType(String),
            const FullType.nullable(JsonObject)
          ]),
          () => MapBuilder<String, JsonObject?>()))
    .build();

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
