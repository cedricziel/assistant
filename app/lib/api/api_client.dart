// app/lib/api/api_client.dart
//
// Wraps the generated assistant_api Dio client and adds SSE streaming support
// for the chat endpoint, which cannot be modelled by openapi-generator.

import 'dart:async';
import 'dart:convert';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/models.dart' show ApprovalDecision;
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:fresh_dio/fresh_dio.dart';
import 'package:one_of/one_of.dart';

import '../features/auth/oauth_credentials.dart';
import 'auth_interceptor.dart';
import 'event_source_stream.dart';
import 'event_source_stream_stub.dart'
    if (dart.library.js_interop) 'event_source_stream_web.dart'
    as event_source;
import 'models/interaction_answer.dart';
import 'models/server_capabilities.dart';
import 'models/stream_event.dart';
import 'native_http_adapter_stub.dart'
    if (dart.library.io) 'native_http_adapter_io.dart'
    as native_http_adapter;
import 'space_header_interceptor.dart';

/// Thrown when the server rejects an API request with HTTP 401.
///
/// Callers can catch this to trigger re-authentication flows or surface a
/// targeted error message rather than a generic "request failed" banner.
class ApiAuthException implements Exception {
  const ApiAuthException([
    this.message = 'Authentication failed — verify your token',
  ]);

  final String message;

  @override
  String toString() => message;
}

/// Configured client bundle: generated API instances + SSE streaming helper.
class ApiClient {
  /// Legacy / static-bearer constructor.
  ///
  /// Used for API-key contexts (`ask_live_...`) and during password-mode
  /// onboarding where the bearer never changes during the session.
  ApiClient({
    required String baseUrl,
    required String token,
    RefreshTokensCallback? refreshTokens,
    OnAuthExpiredCallback? onAuthExpired,
    this.spaceId,
  }) : _staticToken = token,
       _tokenStorage = null {
    _setupDio(baseUrl: baseUrl);

    if (refreshTokens != null && onAuthExpired != null) {
      final interceptor = AuthRecoveryInterceptor(
        dio: _dio,
        refreshTokens: refreshTokens,
        onAuthExpired: onAuthExpired,
      );
      _dio.interceptors.add(interceptor);
    }

    _generatedApi.setBearerAuth('bearer_token', token);
  }

  /// OAuth2 constructor: installs `fresh_dio` so the bearer is fetched from
  /// [tokenStorage] on every request, refreshed proactively when expired, and
  /// retried-once reactively on a 401. [onAuthExpired] fires when refresh
  /// throws [RevokeTokenException] (or fresh_dio otherwise transitions to
  /// `unauthenticated`), so the wiring layer can deactivate the context and
  /// the router can redirect to `/login`.
  ApiClient.oauth2({
    required String baseUrl,
    required TokenStorage<OAuthCredentials> tokenStorage,
    required Future<OAuthCredentials> Function(
      OAuthCredentials? token,
      Dio client,
    )
    refreshToken,
    required OnAuthExpiredCallback onAuthExpired,
    this.spaceId,
  }) : _staticToken = null,
       _tokenStorage = tokenStorage {
    _setupDio(baseUrl: baseUrl);

    // fresh_dio uses a separate Dio for refresh + reactive-401 retry. Passing
    // the main `_dio` would trip its no-self-recursion assertion. We give it a
    // dedicated client with the same baseUrl + native adapter, but NO Fresh
    // interceptor of its own.
    _refreshDio = Dio(BaseOptions(baseUrl: baseUrl));
    native_http_adapter.installNativeHttpAdapter(_refreshDio!);

    final fresh = Fresh<OAuthCredentials>(
      tokenStorage: tokenStorage,
      httpClient: _refreshDio,
      tokenHeader: (creds) => {'Authorization': 'Bearer ${creds.bearerToken}'},
      shouldRefreshBeforeRequest: (_, creds) =>
          creds != null && creds.isExpired(),
      refreshToken: refreshToken,
    );
    _dio.interceptors.add(fresh);

    _authStatusSub = fresh.authenticationStatus.listen((status) {
      if (status == AuthenticationStatus.unauthenticated) {
        onAuthExpired();
      }
    });
  }

  /// Shared Dio + generated-API setup used by both constructors.
  void _setupDio({required String baseUrl}) {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 15),
        receiveTimeout: const Duration(minutes: 10),
      ),
    );

    // Use the platform-native HTTP stack on iOS / macOS (NSURLSession via
    // cupertino_http) and Android (Cronet) so chunked-transfer SSE
    // responses stream in real time. dart:io's HttpClient — Dio's default
    // — buffers SSE bodies on iOS / iPadOS until the connection closes,
    // which is the root cause of the "jumpy dots forever" behaviour
    // mitigated at the application layer by `_recoverStalledStream`.
    //
    // The implementation is selected via conditional import: a no-op stub
    // on web (where `dart.library.io` is unavailable and Dio's
    // BrowserHttpClientAdapter already streams via `fetch`), and the real
    // NativeAdapter on native targets. native_dio_adapter pulls in
    // `cronet_http` → `jni` → `dart:ffi`, so it can only be imported when
    // dart:io is available.
    native_http_adapter.installNativeHttpAdapter(_dio);

    final selectedSpace = spaceId;
    if (selectedSpace != null) {
      _dio.interceptors.add(SpaceHeaderInterceptor(spaceId: selectedSpace));
    }

    _generatedApi = AssistantApi(dio: _dio, basePathOverride: baseUrl);
  }

  /// Static bearer for legacy/API-key contexts; `null` on the OAuth path.
  final String? _staticToken;

  /// Id of the space data-plane calls act in, read on every request; `null`
  /// sends no space selector, which the server accepts only for callers with
  /// a single space.
  final String? Function()? spaceId;

  /// Token storage for the OAuth path; `null` on the legacy path. Used by SSE
  /// methods to read the current bearer (so they pick up refreshed tokens
  /// without rebuilding the client).
  final TokenStorage<OAuthCredentials>? _tokenStorage;

  StreamSubscription<AuthenticationStatus>? _authStatusSub;

  late final Dio _dio;
  late final AssistantApi _generatedApi;

  /// Secondary Dio used by fresh_dio for token refresh and reactive 401
  /// retries. Kept separate from `_dio` to avoid fresh_dio's self-recursion
  /// assertion (an interceptor must not be installed on its own httpClient).
  Dio? _refreshDio;

  @visibleForTesting
  Dio get dioForTesting => _dio;

  /// The underlying HTTP client, for callers that need raw requests (surface
  /// attachment bytes) with the app's interceptors and headers.
  Dio get dio => _dio;

  /// The Dio fresh_dio uses for refresh + retry. Tests inject the same
  /// HttpClientAdapter on both so retries see the mocked responses.
  @visibleForTesting
  Dio? get refreshDioForTesting => _refreshDio;

  /// Disposes the auth-status subscription. Call when the active context
  /// changes; safe to call multiple times.
  void close() {
    _authStatusSub?.cancel();
    _authStatusSub = null;
  }

  /// Returns the bearer the SSE methods should send. Reads from token storage
  /// on the OAuth path so refreshes are picked up automatically; falls back to
  /// the static bearer otherwise.
  Future<String> _currentBearer() async {
    if (_staticToken != null) return _staticToken;
    final creds = await _tokenStorage?.read();
    return creds?.bearerToken ?? '';
  }

  ConversationsApi get conversations => _generatedApi.getConversationsApi();
  AttachmentsApi get attachments => _generatedApi.getAttachmentsApi();
  PersonasApi get personas => _generatedApi.getPersonasApi();
  SkillsApi get skills => _generatedApi.getSkillsApi();
  WebhooksApi get webhooks => _generatedApi.getWebhooksApi();
  AgentsApi get agents => _generatedApi.getAgentsApi();
  WorkflowsApi get workflows => _generatedApi.getWorkflowsApi();
  CapabilitiesApi get capabilities => _generatedApi.getCapabilitiesApi();
  CommandsApi get commands => _generatedApi.getCommandsApi();
  ModelsApi get models => _generatedApi.getModelsApi();
  OrgsApi get orgs => _generatedApi.getOrgsApi();
  SpacesApi get spaces => _generatedApi.getSpacesApi();
  MembersApi get members => _generatedApi.getMembersApi();
  UsersApi get users => _generatedApi.getUsersApi();
  AccountApi get account => _generatedApi.getAccountApi();
  ApiKeysApi get apiKeys => _generatedApi.getApiKeysApi();
  CatalogApi get catalog => _generatedApi.getCatalogApi();
  IntegrationsApi get integrations => _generatedApi.getIntegrationsApi();
  TemplatesApi get templates => _generatedApi.getTemplatesApi();
  ToolPermissionRulesApi get toolPermissionRules =>
      _generatedApi.getToolPermissionRulesApi();

  /// Request that the server cancel an in-flight turn.
  ///
  /// Returns the post-cancel [TurnState] as reported by the server, or
  /// `null` on any error. The endpoint is idempotent — cancelling a
  /// completed / errored / unknown turn is a no-op that simply returns
  /// the current state.
  ///
  /// The actual cancellation propagates asynchronously: the runtime
  /// aborts the in-flight LLM/tool work on its next yield point and the
  /// SSE stream emits a final `agent_error` event with
  /// `{"reason": "cancelled"}`. Callers should still observe the stream
  /// terminating before considering the turn fully reconciled.
  Future<TurnState?> cancelTurn(String conversationId, String turnId) async {
    try {
      final resp = await conversations.cancelTurn(
        conversationId: conversationId,
        turnId: turnId,
      );
      return resp.data?.state;
    } catch (_) {
      return null;
    }
  }

  /// Probe the authoritative server-side state of a turn.
  ///
  /// Returns the [TurnState] from `GET .../turns/{turnId}/status`, or `null`
  /// if the call failed for any reason (network error, deserialization
  /// failure, etc.). Callers treat `null` as "probe inconclusive" and fall
  /// back to whatever they were going to do anyway.
  ///
  /// Used by `ChatNotifier`'s initial-stall watchdog: when the SSE stream
  /// has been quiet for ~12 s but we already have a run id, the probe
  /// disambiguates a slow server (state == running, keep waiting) from
  /// a dead one (completed/errored/unknown, recover).
  Future<TurnState?> turnStatus(String conversationId, String turnId) async {
    try {
      final resp = await conversations.getTurnStatus(
        conversationId: conversationId,
        turnId: turnId,
      );
      return resp.data?.state;
    } catch (_) {
      return null;
    }
  }

  // -- Turn interactions -----------------------------------------------------

  /// Answer a pending interaction of the turn [turnId] (its run id).
  ///
  /// Returns the stored interaction, now `resolved`. Errors surface as
  /// [DioException] so the card can tell a refused answer (400), an interaction
  /// that already ended (404, 409) and an unreachable server apart.
  Future<InteractionDetail> respondToInteraction(
    String conversationId,
    String turnId,
    String interactionId,
    InteractionAnswer answer,
  ) async {
    final resp = await conversations.respondToInteraction(
      conversationId: conversationId,
      turnId: turnId,
      interactionId: interactionId,
      respondToInteractionRequest: _respondRequest(answer),
    );
    final detail = resp.data;
    if (detail == null) {
      throw StateError('respond route returned no interaction');
    }
    return detail;
  }

  /// The interactions of [conversationId], optionally only those in [status].
  ///
  /// Used after a reload to find the approvals and questions that are still
  /// waiting, and to show the receipts of those that ended.
  Future<List<InteractionDetail>> listConversationInteractions(
    String conversationId, {
    InteractionStatusView? status,
  }) async {
    final resp = await conversations.listConversationInteractions(
      conversationId: conversationId,
      status: status,
    );
    return resp.data?.toList() ?? const [];
  }

  // -- Tool permission rules -------------------------------------------------

  /// The saved tool permission rules: the caller's own by default, or with
  /// [RuleScope.space] every member's (admins only).
  Future<List<ToolPermissionRuleSummary>> listToolPermissionRules({
    RuleScope? scope,
  }) async {
    final resp = await toolPermissionRules.listToolPermissionRules(
      scope: scope,
    );
    return resp.data?.toList() ?? const [];
  }

  /// Revoke the saved rule [id]. Throws [DioException] when it is gone or not
  /// the caller's to revoke.
  Future<void> deleteToolPermissionRule(String id) async {
    await toolPermissionRules.deleteToolPermissionRule(id: id);
  }

  /// Stream assistant tokens for a conversation message.
  ///
  /// Yields [RunStartedEvent] first (from the `X-Run-Id` response header),
  /// then yields [TokenEvent]s for each incremental chunk and a final
  /// [DoneEvent] when the server closes the stream. A rejected request yields
  /// a single [ErrorEvent] carrying the HTTP status (409 while an edit or
  /// regenerate holds the conversation).
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) {
    final body = <String, dynamic>{'message': message};
    if (attachmentIds != null && attachmentIds.isNotEmpty) {
      body['attachment_ids'] = attachmentIds;
    }
    return _postSse('/api/conversations/$conversationId/messages', body);
  }

  /// Edit the user message [messageId] to [text] and stream the reply on the
  /// new branch. Same frames as [streamMessages]; a 409 yields an
  /// [ErrorEvent] with `isConflict`.
  Stream<StreamEvent> streamEdit(
    String conversationId,
    String messageId,
    String text,
  ) => _postSse('/api/conversations/$conversationId/messages/$messageId/edit', {
    'message': text,
  });

  /// Regenerate the reply to [messageId] (a user message or any reply row),
  /// optionally with another [model]. Same frames as [streamMessages]. A 404
  /// yields an [ErrorEvent] with `isNotFound`: the run failed before saving
  /// the user row, so callers fall back to re-sending.
  Stream<StreamEvent> streamRegenerate(
    String conversationId,
    String messageId, {
    String? model,
  }) => _postSse(
    '/api/conversations/$conversationId/messages/$messageId/regenerate',
    {'model': ?model},
  );

  /// POSTs [body] as JSON to [path] and parses the SSE response.
  Stream<StreamEvent> _postSse(String path, Map<String, dynamic> body) async* {
    final bearer = await _currentBearer();
    final Response<ResponseBody> response;
    try {
      response = await _dio.post<ResponseBody>(
        path,
        data: jsonEncode(body),
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Content-Type': 'application/json',
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $bearer',
          },
        ),
      );
    } on DioException catch (e) {
      yield ErrorEvent(
        e.message ?? 'Request failed',
        statusCode: e.response?.statusCode,
      );
      return;
    }

    // Eagerly surface the run ID from the response header so callers can
    // capture it before the first SSE event arrives.
    final runIdHeader = response.headers.value('X-Run-Id');
    if (runIdHeader != null) {
      yield RunStartedEvent(runIdHeader);
    }

    yield* _parseSse(withHeartbeatTimeout(response.data!.stream));
  }

  /// Replay events for an existing orchestrator run.
  ///
  /// Calls `GET /api/conversations/{id}/runs/{runId}/events/stream?since={since}`
  /// and streams the SSE response.  Throws [DioException] with status 404 if
  /// the run is unknown or 410 if its events were pruned. Cancelling
  /// [cancelToken] closes the connection.
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) async* {
    final bearer = await _currentBearer();
    final Response<ResponseBody> response;
    try {
      response = await _dio.get<ResponseBody>(
        '/api/conversations/$conversationId/runs/$runId/events/stream',
        queryParameters: {'since': since},
        cancelToken: cancelToken,
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $bearer',
          },
        ),
      );
    } on DioException {
      rethrow; // 404 / 410 propagate to caller
    }

    yield* _parseSse(withHeartbeatTimeout(response.data!.stream));
  }

  // -- Voice / capabilities ---------------------------------------------------

  /// Fetch server capability flags (voice_send, voice_receive, …).
  Future<ServerCapabilities> getCapabilities({CancelToken? cancelToken}) async {
    try {
      final response = await capabilities.getCapabilities(
        cancelToken: cancelToken,
      );
      final data = response.data;
      if (data != null) {
        return ServerCapabilities(
          voiceSend: data.voiceSend,
          voiceReceive: data.voiceReceive,
          linkHosts: data.ui.linkHosts.toList(),
        );
      }
    } catch (_) {
      // Network, cancellation, or parse errors → report no capabilities.
    }
    return ServerCapabilities.disabled;
  }

  /// Upload [audioBytes] to the voice endpoint and stream the SSE response.
  Stream<StreamEvent> sendVoiceMessage(
    String conversationId,
    Uint8List audioBytes,
    String mimeType,
  ) async* {
    final extension = switch (mimeType) {
      String m when m.contains('webm') => 'webm',
      String m when m.contains('ogg') => 'ogg',
      String m when m.contains('wav') => 'wav',
      String m when m.contains('aac') => 'aac',
      _ => 'm4a',
    };
    final formData = FormData.fromMap({
      'audio': MultipartFile.fromBytes(
        audioBytes,
        filename: 'audio.$extension',
        contentType: DioMediaType.parse(mimeType),
      ),
    });

    final bearer = await _currentBearer();
    final Response<ResponseBody> response;
    try {
      response = await _dio.post<ResponseBody>(
        '/api/conversations/$conversationId/voice',
        data: formData,
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $bearer',
          },
        ),
      );
    } on DioException catch (e) {
      yield ErrorEvent(e.message ?? 'Voice upload failed');
      return;
    }

    // Same as `streamMessages`: surface the run ID before any body frame so
    // Stop can cancel the turn even while the body is still buffered.
    final runIdHeader = response.headers.value('X-Run-Id');
    if (runIdHeader != null) {
      yield RunStartedEvent(runIdHeader);
    }

    yield* _parseSse(withHeartbeatTimeout(response.data!.stream));
  }

  /// Fetch the audio bytes for a message (GET /api/messages/{id}/audio).
  Future<({Uint8List bytes, String mimeType})?> fetchMessageAudio(
    String messageId,
  ) async {
    try {
      final bearer = await _currentBearer();
      final response = await _dio.get<List<int>>(
        '/api/messages/$messageId/audio',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $bearer'},
        ),
      );
      if (response.statusCode == 200 && response.data != null) {
        final mime = response.headers.value('content-type') ?? 'audio/mpeg';
        return (bytes: Uint8List.fromList(response.data!), mimeType: mime);
      }
    } catch (_) {
      // ignore
    }
    return null;
  }

  /// Fetch audio bytes by audio ID (GET /api/audio/{id}).
  Future<({Uint8List bytes, String mimeType})?> fetchAudio(
    String audioId,
  ) async {
    try {
      final bearer = await _currentBearer();
      final response = await _dio.get<List<int>>(
        '/api/audio/$audioId',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $bearer'},
        ),
      );
      if (response.statusCode == 200 && response.data != null) {
        final mime = response.headers.value('content-type') ?? 'audio/mpeg';
        return (bytes: Uint8List.fromList(response.data!), mimeType: mime);
      }
    } catch (_) {
      // ignore
    }
    return null;
  }

  // -- Conversation list stream -----------------------------------------------

  /// Stream conversation list changes via SSE.
  ///
  /// Yields [ConversationSnapshotEvent] first (full list), then
  /// [ConversationUpsertedEvent] and [ConversationDeletedEvent] deltas.
  ///
  /// On web, uses the browser's native `EventSource` API (because dio's
  /// browser adapter buffers stream responses until the connection closes,
  /// which never happens for SSE — see `web-sse-eventsource` change). On
  /// native, uses the existing dio stream path.
  Stream<ConversationListEvent> streamConversations({String? agentId}) {
    if (kIsWeb) {
      return _streamConversationsViaEventSource(agentId: agentId);
    }
    return _streamConversationsViaDio(agentId: agentId);
  }

  Stream<ConversationListEvent> _streamConversationsViaEventSource({
    String? agentId,
  }) async* {
    final base = _dio.options.baseUrl;
    final bearer = await _currentBearer();
    final qp = <String, String>{
      'access_token': bearer,
      // The browser EventSource cannot set X-Space-Id.
      spaceIdQueryParam: ?spaceId?.call(),
      // ignore: use_null_aware_elements — value is nullable, key is not.
      if (agentId != null) 'agent_id': agentId,
    };
    final query = qp.entries
        .map(
          (e) =>
              '${Uri.encodeQueryComponent(e.key)}=${Uri.encodeQueryComponent(e.value)}',
        )
        .join('&');
    final url = '$base/api/conversations/stream?$query';
    final adapter = event_source.openEventSourceAdapter(url: url);
    yield* openConversationListStream(adapter: adapter);
  }

  Stream<ConversationListEvent> _streamConversationsViaDio({
    String? agentId,
  }) async* {
    final queryParams = <String, dynamic>{};
    if (agentId != null) queryParams['agent_id'] = agentId;

    final bearer = await _currentBearer();
    final Response<ResponseBody> response;
    try {
      response = await _dio.get<ResponseBody>(
        '/api/conversations/stream',
        queryParameters: queryParams,
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $bearer',
          },
        ),
      );
    } on DioException catch (e) {
      if (e.response?.statusCode == 401) {
        throw const ApiAuthException();
      }
      throw Exception('Failed to connect to conversation stream: ${e.message}');
    }

    yield* _parseConversationSse(withHeartbeatTimeout(response.data!.stream));
  }

  // -- SSE parser -------------------------------------------------------------

  Stream<StreamEvent> _parseSse(Stream<List<int>> byteStream) =>
      parseSseByteStream(byteStream);

  Stream<ConversationListEvent> _parseConversationSse(
    Stream<List<int>> byteStream,
  ) => parseConversationSseByteStream(byteStream);
}

typedef _RespondOneOf =
    OneOf4<
      RespondToInteractionRequestOneOf,
      RespondToInteractionRequestOneOf1,
      RespondToInteractionRequestOneOf2,
      RespondToInteractionRequestOneOf3
    >;

/// Builds the generated respond-route body for [answer].
RespondToInteractionRequest _respondRequest(InteractionAnswer answer) =>
    RespondToInteractionRequest(
      (b) => b.oneOf = switch (answer) {
        ToolApprovalAnswer(:final decision) => _RespondOneOf(
          typeIndex: 0,
          value: RespondToInteractionRequestOneOf(
            (r) => r
              ..kind = RespondToInteractionRequestOneOfKindEnum.toolApproval
              ..decision = switch (decision) {
                ApprovalDecision.allowOnce => ToolDecision.allowOnce,
                ApprovalDecision.allowAlways => ToolDecision.allowAlways,
                ApprovalDecision.rejectOnce => ToolDecision.rejectOnce,
                ApprovalDecision.rejectAlways => ToolDecision.rejectAlways,
              },
          ),
        ),
        QuestionAnswers(:final answers) => _RespondOneOf(
          typeIndex: 1,
          value: RespondToInteractionRequestOneOf1(
            (r) => r
              ..kind = RespondToInteractionRequestOneOf1KindEnum.question
              ..answers.replace([
                for (final a in answers)
                  QuestionAnswer(
                    (q) => q
                      ..questionId = a.questionId
                      ..selectedOptionIds.replace(a.selectedOptionIds)
                      ..text = a.text,
                  ),
              ]),
          ),
        ),
        DeclineAnswer() => _RespondOneOf(
          typeIndex: 3,
          value: RespondToInteractionRequestOneOf3(
            (r) => r.kind = RespondToInteractionRequestOneOf3KindEnum.decline,
          ),
        ),
      },
    );

// -- Heartbeat timeout wrapper ------------------------------------------------

/// Wraps an SSE byte stream with a watchdog timer that fires a
/// [TimeoutException] if no data arrives within [timeout].
///
/// The Axum backend applies `KeepAlive::default()` to every `Sse` response
/// (see `crates/web-ui/src/api/mod.rs::sse_response`), which emits a `:`
/// comment line every 15 seconds during periods of byte silence. The
/// watchdog resets on **any** byte arriving on the underlying stream —
/// including those comment bytes — so a healthy slow tool call that emits
/// no semantic events for minutes does NOT trip this timeout.
///
/// A 90-second default tolerates ~6 missed keep-alive intervals before
/// declaring the connection genuinely stale.
///
/// Contract lock: this reset-on-any-byte behaviour is covered by
/// `app/test/unit/api/heartbeat_timeout_test.dart` and the end-to-end
/// keep-alive wire contract is locked by
/// `crates/web-ui/tests/sse_keepalive_contract.rs`.
Stream<List<int>> withHeartbeatTimeout(
  Stream<List<int>> source, {
  Duration timeout = const Duration(seconds: 90),
}) {
  late StreamController<List<int>> controller;
  StreamSubscription<List<int>>? subscription;
  Timer? watchdog;

  void resetWatchdog() {
    watchdog?.cancel();
    watchdog = Timer(timeout, () {
      controller.addError(
        TimeoutException('No data received within $timeout', timeout),
      );
      subscription?.cancel();
      controller.close();
    });
  }

  controller = StreamController<List<int>>(
    onListen: () {
      resetWatchdog();
      subscription = source.listen(
        (data) {
          resetWatchdog();
          controller.add(data);
        },
        onError: (Object e, StackTrace st) {
          watchdog?.cancel();
          controller.addError(e, st);
        },
        onDone: () {
          watchdog?.cancel();
          controller.close();
        },
      );
    },
    onCancel: () {
      watchdog?.cancel();
      watchdog = null;
      return subscription?.cancel();
    },
  );

  return controller.stream;
}

// -- SSE parser (top-level for testability) -----------------------------------

/// Parses a raw SSE byte stream into [StreamEvent]s.
///
/// Accepts [Stream<List<int>>] or [Stream<Uint8List>] (both are [List<int>]).
/// Uses [utf8.decoder.bind] for correct handling of multi-byte characters
/// that may be split across chunk boundaries.
Stream<StreamEvent> parseSseByteStream(Stream<List<int>> byteStream) async* {
  final lines = utf8.decoder.bind(byteStream).transform(const LineSplitter());

  String? eventType;
  String? dataLine;
  String? eventId;

  await for (final line in lines) {
    if (line.startsWith('event:')) {
      eventType = line.substring('event:'.length).trim();
    } else if (line.startsWith('data:')) {
      // Per the SSE spec, multiple `data:` lines are concatenated with '\n'.
      final value = line.substring('data:'.length).trim();
      dataLine = dataLine == null ? value : '$dataLine\n$value';
    } else if (line.startsWith('id:')) {
      eventId = line.substring('id:'.length).trim();
    } else if (line.isEmpty) {
      // Blank line — dispatch the event.
      final seq = int.tryParse(eventId ?? '');
      if (eventType == 'token' && dataLine != null) {
        yield TokenEvent(dataLine, sequenceId: seq);
      } else if (eventType == 'status' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield StatusEvent(
            json['message'] as String? ?? '',
            toolCallId: json['tool_call_id'] as String?,
            sequenceId: seq,
          );
        } catch (_) {
          // Older servers send plain-text status messages.
          yield StatusEvent(dataLine, sequenceId: seq);
        }
      } else if (eventType == 'tool_result' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield ToolResultEvent(
            toolName: json['tool_name'] as String? ?? '',
            status: json['status'] as String? ?? 'ok',
            arguments: json['arguments'] as Map<String, dynamic>?,
            result: json['result'] as String?,
            toolCallId: json['tool_call_id'] as String?,
            data: json['data'] is Map
                ? (json['data'] as Map).cast<String, Object?>()
                : null,
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed tool_result events
        }
      } else if (eventType == 'run_started' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield RunStartedEvent(
            json['run_id'] as String? ?? '',
            userMessageId: json['user_message_id'] as String?,
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed run_started events
        }
      } else if (eventType == 'done' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield DoneEvent(
            role: json['role'] as String? ?? 'assistant',
            content: json['content'] as String? ?? '',
            messageId: json['message_id'] as String?,
            timing: messageTimingFromJson(json['timing']),
            sequenceId: seq,
          );
        } catch (_) {
          yield DoneEvent(
            role: 'assistant',
            content: dataLine,
            sequenceId: seq,
          );
        }
      } else if (eventType == 'transcript' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield TranscriptEvent(
            json['content'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed transcript events
        }
      } else if (eventType == 'thinking' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield ThinkingEvent(
            json['content'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed thinking events
        }
      } else if (eventType == 'subagent_started' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield SubagentStartedEvent(
            agentId: json['agent_id'] as String? ?? '',
            task: json['task'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_started events
        }
      } else if (eventType == 'subagent_completed' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield SubagentCompletedEvent(
            agentId: json['agent_id'] as String? ?? '',
            status: json['status'] as String? ?? 'ok',
            summary: json['summary'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_completed events
        }
      } else if (eventType == 'skill_complete' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield SkillCompleteEvent(
            skillName: json['skill_name'] as String? ?? '',
            success: json['success'] as bool? ?? false,
            summary: json['summary'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed skill_complete events
        }
      } else if (eventType == 'subagent_token' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          final data = json['data'] as Map<String, dynamic>? ?? {};
          yield SubagentTokenEvent(
            agentId: json['agent_id'] as String? ?? '',
            content: data['content'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_token events
        }
      } else if (eventType == 'subagent_thinking' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          final data = json['data'] as Map<String, dynamic>? ?? {};
          yield SubagentThinkingEvent(
            agentId: json['agent_id'] as String? ?? '',
            content: data['content'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_thinking events
        }
      } else if (eventType == 'subagent_tool_result' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          final data = json['data'] as Map<String, dynamic>? ?? {};
          yield SubagentToolResultEvent(
            agentId: json['agent_id'] as String? ?? '',
            toolName: data['tool_name'] as String? ?? '',
            status: data['status'] as String? ?? 'ok',
            arguments: data['arguments'] as Map<String, dynamic>?,
            result: data['result'] as String?,
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_tool_result events
        }
      } else if (eventType == 'subagent_status' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          final data = json['data'] as Map<String, dynamic>? ?? {};
          yield SubagentStatusEvent(
            agentId: json['agent_id'] as String? ?? '',
            message: data['message'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed subagent_status events
        }
      } else if (eventType == 'agent_error' && dataLine != null) {
        yield AgentErrorEvent.fromData(dataLine, sequenceId: seq);
      } else if (eventType == 'audio_ready' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield AudioReadyEvent(
            json['audio_id'] as String? ?? '',
            sequenceId: seq,
          );
        } catch (_) {
          // ignore malformed audio_ready events
        }
      } else if (eventType == 'suggestions' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield SuggestionsEvent.fromJson(json, sequenceId: seq);
        } catch (_) {
          // ignore malformed suggestions events
        }
      } else if (eventType == 'interaction_requested' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield InteractionRequestedEvent.fromJson(json, sequenceId: seq);
        } catch (_) {
          // ignore malformed interaction_requested events
        }
      } else if (eventType == 'interaction_resolved' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield InteractionResolvedEvent.fromJson(json, sequenceId: seq);
        } catch (_) {
          // ignore malformed interaction_resolved events
        }
      } else if (eventType == 'surface_update' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield SurfaceUpdateEvent.fromJson(json, sequenceId: seq);
        } catch (_) {
          // ignore malformed surface_update events
        }
      }
      eventType = null;
      dataLine = null;
      eventId = null;
    }
  }
}

/// Parses a raw SSE byte stream into [ConversationListEvent]s.
Stream<ConversationListEvent> parseConversationSseByteStream(
  Stream<List<int>> byteStream,
) async* {
  final lines = utf8.decoder.bind(byteStream).transform(const LineSplitter());

  String? eventType;
  String? dataLine;

  await for (final line in lines) {
    if (line.startsWith('event:')) {
      eventType = line.substring('event:'.length).trim();
    } else if (line.startsWith('data:')) {
      final value = line.substring('data:'.length).trim();
      dataLine = dataLine == null ? value : '$dataLine\n$value';
    } else if (line.startsWith('id:')) {
      // Consume but ignore — conversation list events don't use sequence IDs.
    } else if (line.isEmpty) {
      if (eventType != null && dataLine != null) {
        try {
          switch (eventType) {
            case 'snapshot':
              final json = jsonDecode(dataLine) as List<dynamic>;
              yield ConversationSnapshotEvent.fromJson(json);
            case 'upserted':
              final json = jsonDecode(dataLine) as Map<String, dynamic>;
              yield ConversationUpsertedEvent.fromJson(json);
            case 'deleted':
              final json = jsonDecode(dataLine) as Map<String, dynamic>;
              yield ConversationDeletedEvent.fromJson(json);
          }
        } catch (_) {
          // ignore malformed conversation events
        }
      }
      eventType = null;
      dataLine = null;
    }
  }
}
