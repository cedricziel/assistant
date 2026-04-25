//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

import 'package:dio/dio.dart';
import 'package:built_value/serializer.dart';
import 'package:assistant_api/src/serializers.dart';
import 'package:assistant_api/src/auth/api_key_auth.dart';
import 'package:assistant_api/src/auth/basic_auth.dart';
import 'package:assistant_api/src/auth/bearer_auth.dart';
import 'package:assistant_api/src/auth/oauth.dart';
import 'package:assistant_api/src/api/agent_card_api.dart';
import 'package:assistant_api/src/api/agents_api.dart';
import 'package:assistant_api/src/api/analytics_api.dart';
import 'package:assistant_api/src/api/api_keys_api.dart';
import 'package:assistant_api/src/api/attachments_api.dart';
import 'package:assistant_api/src/api/bindings_api.dart';
import 'package:assistant_api/src/api/capabilities_api.dart';
import 'package:assistant_api/src/api/catalog_api.dart';
import 'package:assistant_api/src/api/commands_api.dart';
import 'package:assistant_api/src/api/conversations_api.dart';
import 'package:assistant_api/src/api/interfaces_api.dart';
import 'package:assistant_api/src/api/logs_api.dart';
import 'package:assistant_api/src/api/members_api.dart';
import 'package:assistant_api/src/api/messages_api.dart';
import 'package:assistant_api/src/api/oauth_api.dart';
import 'package:assistant_api/src/api/orgs_api.dart';
import 'package:assistant_api/src/api/personas_api.dart';
import 'package:assistant_api/src/api/push_notifications_api.dart';
import 'package:assistant_api/src/api/skills_api.dart';
import 'package:assistant_api/src/api/spaces_api.dart';
import 'package:assistant_api/src/api/tasks_api.dart';
import 'package:assistant_api/src/api/templates_api.dart';
import 'package:assistant_api/src/api/traces_api.dart';
import 'package:assistant_api/src/api/users_api.dart';
import 'package:assistant_api/src/api/web_push_api.dart';
import 'package:assistant_api/src/api/webhooks_api.dart';
import 'package:assistant_api/src/api/workflows_api.dart';

class AssistantApi {
  static const String basePath = r'http://localhost';

  final Dio dio;
  final Serializers serializers;

  AssistantApi({
    Dio? dio,
    Serializers? serializers,
    String? basePathOverride,
    List<Interceptor>? interceptors,
  })  : this.serializers = serializers ?? standardSerializers,
        this.dio = dio ??
            Dio(BaseOptions(
              baseUrl: basePathOverride ?? basePath,
              connectTimeout: const Duration(milliseconds: 5000),
              receiveTimeout: const Duration(milliseconds: 3000),
            )) {
    if (interceptors == null) {
      this.dio.interceptors.addAll([
        OAuthInterceptor(),
        BasicAuthInterceptor(),
        BearerAuthInterceptor(),
        ApiKeyAuthInterceptor(),
      ]);
    } else {
      this.dio.interceptors.addAll(interceptors);
    }
  }

  void setOAuthToken(String name, String token) {
    if (this.dio.interceptors.any((i) => i is OAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is OAuthInterceptor)
              as OAuthInterceptor)
          .tokens[name] = token;
    }
  }

  void setBearerAuth(String name, String token) {
    if (this.dio.interceptors.any((i) => i is BearerAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BearerAuthInterceptor)
              as BearerAuthInterceptor)
          .tokens[name] = token;
    }
  }

  void setBasicAuth(String name, String username, String password) {
    if (this.dio.interceptors.any((i) => i is BasicAuthInterceptor)) {
      (this.dio.interceptors.firstWhere((i) => i is BasicAuthInterceptor)
              as BasicAuthInterceptor)
          .authInfo[name] = BasicAuthInfo(username, password);
    }
  }

  void setApiKey(String name, String apiKey) {
    if (this.dio.interceptors.any((i) => i is ApiKeyAuthInterceptor)) {
      (this
                  .dio
                  .interceptors
                  .firstWhere((element) => element is ApiKeyAuthInterceptor)
              as ApiKeyAuthInterceptor)
          .apiKeys[name] = apiKey;
    }
  }

  /// Get AgentCardApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AgentCardApi getAgentCardApi() {
    return AgentCardApi(dio, serializers);
  }

  /// Get AgentsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AgentsApi getAgentsApi() {
    return AgentsApi(dio, serializers);
  }

  /// Get AnalyticsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AnalyticsApi getAnalyticsApi() {
    return AnalyticsApi(dio, serializers);
  }

  /// Get ApiKeysApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ApiKeysApi getApiKeysApi() {
    return ApiKeysApi(dio, serializers);
  }

  /// Get AttachmentsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  AttachmentsApi getAttachmentsApi() {
    return AttachmentsApi(dio, serializers);
  }

  /// Get BindingsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  BindingsApi getBindingsApi() {
    return BindingsApi(dio, serializers);
  }

  /// Get CapabilitiesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  CapabilitiesApi getCapabilitiesApi() {
    return CapabilitiesApi(dio, serializers);
  }

  /// Get CatalogApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  CatalogApi getCatalogApi() {
    return CatalogApi(dio, serializers);
  }

  /// Get CommandsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  CommandsApi getCommandsApi() {
    return CommandsApi(dio, serializers);
  }

  /// Get ConversationsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  ConversationsApi getConversationsApi() {
    return ConversationsApi(dio, serializers);
  }

  /// Get InterfacesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  InterfacesApi getInterfacesApi() {
    return InterfacesApi(dio, serializers);
  }

  /// Get LogsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  LogsApi getLogsApi() {
    return LogsApi(dio, serializers);
  }

  /// Get MembersApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  MembersApi getMembersApi() {
    return MembersApi(dio, serializers);
  }

  /// Get MessagesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  MessagesApi getMessagesApi() {
    return MessagesApi(dio, serializers);
  }

  /// Get OauthApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  OauthApi getOauthApi() {
    return OauthApi(dio, serializers);
  }

  /// Get OrgsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  OrgsApi getOrgsApi() {
    return OrgsApi(dio, serializers);
  }

  /// Get PersonasApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  PersonasApi getPersonasApi() {
    return PersonasApi(dio, serializers);
  }

  /// Get PushNotificationsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  PushNotificationsApi getPushNotificationsApi() {
    return PushNotificationsApi(dio, serializers);
  }

  /// Get SkillsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  SkillsApi getSkillsApi() {
    return SkillsApi(dio, serializers);
  }

  /// Get SpacesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  SpacesApi getSpacesApi() {
    return SpacesApi(dio, serializers);
  }

  /// Get TasksApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  TasksApi getTasksApi() {
    return TasksApi(dio, serializers);
  }

  /// Get TemplatesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  TemplatesApi getTemplatesApi() {
    return TemplatesApi(dio, serializers);
  }

  /// Get TracesApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  TracesApi getTracesApi() {
    return TracesApi(dio, serializers);
  }

  /// Get UsersApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  UsersApi getUsersApi() {
    return UsersApi(dio, serializers);
  }

  /// Get WebPushApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  WebPushApi getWebPushApi() {
    return WebPushApi(dio, serializers);
  }

  /// Get WebhooksApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  WebhooksApi getWebhooksApi() {
    return WebhooksApi(dio, serializers);
  }

  /// Get WorkflowsApi instance, base route and serializer can be overridden by a given but be careful,
  /// by doing that all interceptors will not be executed
  WorkflowsApi getWorkflowsApi() {
    return WorkflowsApi(dio, serializers);
  }
}
