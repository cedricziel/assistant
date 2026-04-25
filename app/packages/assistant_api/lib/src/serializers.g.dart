// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'serializers.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

Serializers _$serializers = (Serializers().toBuilder()
      ..add(AddMemberRequest.serializer)
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
      ..add(ApiKeySummary.serializer)
      ..add(ApiSendMessageRequest.serializer)
      ..add(Artifact.serializer)
      ..add(AttachmentMetaResponse.serializer)
      ..add(AuthenticationInfo.serializer)
      ..add(AuthorizationCodeOAuthFlow.serializer)
      ..add(BindingResponse.serializer)
      ..add(CancelTaskRequest.serializer)
      ..add(CatalogItemResponse.serializer)
      ..add(ClientCredentialsOAuthFlow.serializer)
      ..add(ClientInfoSchema.serializer)
      ..add(ClientRegistrationSchema.serializer)
      ..add(CommandArgResponse.serializer)
      ..add(CommandDefResponse.serializer)
      ..add(CommandEventResponse.serializer)
      ..add(ConversationDetail.serializer)
      ..add(ConversationSummary.serializer)
      ..add(CreateApiKeyRequest.serializer)
      ..add(CreateApiKeyResponse.serializer)
      ..add(CreateBindingRequest.serializer)
      ..add(CreateConversationRequest.serializer)
      ..add(CreateFromTemplateRequest.serializer)
      ..add(CreateInterfaceInstanceRequest.serializer)
      ..add(CreateOrgRequest.serializer)
      ..add(CreatePersonaRequest.serializer)
      ..add(CreateSkillRequest.serializer)
      ..add(CreateSpaceRequest.serializer)
      ..add(CreateSubscriptionRequest.serializer)
      ..add(CreateTaskPushNotificationConfigRequest.serializer)
      ..add(CreateUserRequest.serializer)
      ..add(CreateWebhookRequest.serializer)
      ..add(DeviceCodeOAuthFlow.serializer)
      ..add(DeviceCodeResponseSchema.serializer)
      ..add(ExecuteCommandRequest.serializer)
      ..add(HttpAuthSecurityScheme.serializer)
      ..add(ImplicitOAuthFlow.serializer)
      ..add(InterfaceInstanceResponse.serializer)
      ..add(ListTaskPushNotificationConfigsResponse.serializer)
      ..add(ListTasksResponse.serializer)
      ..add(LogEntryResponse.serializer)
      ..add(MemberEntry.serializer)
      ..add(Message.serializer)
      ..add(MessageSummary.serializer)
      ..add(ModelPart.serializer)
      ..add(ModelUsageResponse.serializer)
      ..add(MutualTlsSecurityScheme.serializer)
      ..add(OAuth2SecurityScheme.serializer)
      ..add(OAuthErrorResponse.serializer)
      ..add(OAuthFlows.serializer)
      ..add(OnboardingStatusResponse.serializer)
      ..add(OpenIdConnectSecurityScheme.serializer)
      ..add(OrgDetail.serializer)
      ..add(OrgSummary.serializer)
      ..add(PasswordOAuthFlow.serializer)
      ..add(PersonaDetail.serializer)
      ..add(PersonaFileContent.serializer)
      ..add(PersonaFileSlot.serializer)
      ..add(PersonaFromTemplateResponse.serializer)
      ..add(PersonaSkillAccess.serializer)
      ..add(PersonaSummary.serializer)
      ..add(PublishCatalogItemRequest.serializer)
      ..add(PushNotificationConfig.serializer)
      ..add(QuickMessageRequest.serializer)
      ..add(QuickMessageResponse.serializer)
      ..add(RegisterAgentRequest.serializer)
      ..add(Role.serializer)
      ..add(RotateSecretResponse.serializer)
      ..add(SecurityRequirement.serializer)
      ..add(SecurityScheme.serializer)
      ..add(SendMessageConfiguration.serializer)
      ..add(SendMessageRequest.serializer)
      ..add(SendMessageResponse.serializer)
      ..add(ServerCapabilities.serializer)
      ..add(ServerMetadata.serializer)
      ..add(SetActivePersonaRequest.serializer)
      ..add(SetSkillAccessModeRequest.serializer)
      ..add(SkillDetail.serializer)
      ..add(SkillEntryResponse.serializer)
      ..add(SpaceDetail.serializer)
      ..add(SpaceSummary.serializer)
      ..add(SpanEntryResponse.serializer)
      ..add(SseStatusEvent.serializer)
      ..add(SseSubagentCompletedEvent.serializer)
      ..add(SseSubagentStartedEvent.serializer)
      ..add(SseSubagentStatusEvent.serializer)
      ..add(SseSubagentThinkingEvent.serializer)
      ..add(SseSubagentTokenEvent.serializer)
      ..add(SseSubagentToolResultEvent.serializer)
      ..add(SseThinkingEvent.serializer)
      ..add(SseTokenEvent.serializer)
      ..add(SseToolResultEvent.serializer)
      ..add(StreamResponse.serializer)
      ..add(StreamRunEventsQuery.serializer)
      ..add(StringList.serializer)
      ..add(SubscribeRequest.serializer)
      ..add(SubscriptionResponse.serializer)
      ..add(Task.serializer)
      ..add(TaskArtifactUpdateEvent.serializer)
      ..add(TaskPushNotificationConfig.serializer)
      ..add(TaskState.serializer)
      ..add(TaskStatus.serializer)
      ..add(TaskStatusUpdateEvent.serializer)
      ..add(TemplateResponse.serializer)
      ..add(TimeSeriesResponse.serializer)
      ..add(TokenResponse.serializer)
      ..add(ToolCallSummary.serializer)
      ..add(ToolUsageResponse.serializer)
      ..add(TraceDetailResponse.serializer)
      ..add(TraceSummaryResponse.serializer)
      ..add(UnsubscribeRequest.serializer)
      ..add(UpdateAgentRequest.serializer)
      ..add(UpdateConversationRequest.serializer)
      ..add(UpdateMemberRequest.serializer)
      ..add(UpdateOrgRequest.serializer)
      ..add(UpdateSkillRequest.serializer)
      ..add(UpdateSpaceRequest.serializer)
      ..add(UpdateUserRequest.serializer)
      ..add(UpdateWebhookRequest.serializer)
      ..add(UserDetail.serializer)
      ..add(UserSummary.serializer)
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
      ..add(WorkflowWebhookTriggerAccepted.serializer)
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
          const FullType(
              BuiltList, const [const FullType(AttachmentMetaResponse)]),
          () => ListBuilder<AttachmentMetaResponse>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(ToolCallSummary)]),
          () => ListBuilder<ToolCallSummary>())
      ..addBuilderFactory(
          const FullType(BuiltList, const [const FullType(CommandArgResponse)]),
          () => ListBuilder<CommandArgResponse>())
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
