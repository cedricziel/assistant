// app/lib/api/api_client.dart
//
// Wraps the generated assistant_api Dio client and adds SSE streaming support
// for the chat endpoint, which cannot be modelled by openapi-generator.

import 'dart:async';
import 'dart:convert';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';

import 'auth_interceptor.dart';
import 'event_source_stream.dart';
import 'event_source_stream_stub.dart'
    if (dart.library.js_interop) 'event_source_stream_web.dart'
    as event_source;
import 'models/server_capabilities.dart';
import 'models/stream_event.dart';

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
  ApiClient({
    required String baseUrl,
    required String token,
    RefreshTokensCallback? refreshTokens,
    OnAuthExpiredCallback? onAuthExpired,
  }) : _token = token {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 15),
        receiveTimeout: const Duration(minutes: 10),
      ),
    );

    if (refreshTokens != null && onAuthExpired != null) {
      final interceptor = AuthRecoveryInterceptor(
        dio: _dio,
        refreshTokens: refreshTokens,
        onAuthExpired: onAuthExpired,
      );
      _dio.interceptors.add(interceptor);
    }

    _generatedApi = AssistantApi(dio: _dio, basePathOverride: baseUrl);
    _generatedApi.setBearerAuth('bearer_token', token);
  }

  final String _token;
  late final Dio _dio;
  late final AssistantApi _generatedApi;

  ConversationsApi get conversations => _generatedApi.getConversationsApi();
  AttachmentsApi get attachments => _generatedApi.getAttachmentsApi();
  PersonasApi get personas => _generatedApi.getPersonasApi();
  SkillsApi get skills => _generatedApi.getSkillsApi();
  LogsApi get logs => _generatedApi.getLogsApi();
  TracesApi get traces => _generatedApi.getTracesApi();
  WebhooksApi get webhooks => _generatedApi.getWebhooksApi();
  AgentsApi get agents => _generatedApi.getAgentsApi();
  AnalyticsApi get analytics => _generatedApi.getAnalyticsApi();
  WorkflowsApi get workflows => _generatedApi.getWorkflowsApi();
  CapabilitiesApi get capabilities => _generatedApi.getCapabilitiesApi();
  CommandsApi get commands => _generatedApi.getCommandsApi();
  OrgsApi get orgs => _generatedApi.getOrgsApi();
  SpacesApi get spaces => _generatedApi.getSpacesApi();
  MembersApi get members => _generatedApi.getMembersApi();
  UsersApi get users => _generatedApi.getUsersApi();
  AccountApi get account => _generatedApi.getAccountApi();
  ApiKeysApi get apiKeys => _generatedApi.getApiKeysApi();
  CatalogApi get catalog => _generatedApi.getCatalogApi();
  TemplatesApi get templates => _generatedApi.getTemplatesApi();

  /// Stream assistant tokens for a conversation message.
  ///
  /// Yields [RunStartedEvent] first (from the `X-Run-Id` response header),
  /// then yields [TokenEvent]s for each incremental chunk and a final
  /// [DoneEvent] when the server closes the stream.
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) async* {
    final body = <String, dynamic>{'message': message};
    if (attachmentIds != null && attachmentIds.isNotEmpty) {
      body['attachment_ids'] = attachmentIds;
    }
    final Response<ResponseBody> response;
    try {
      response = await _dio.post<ResponseBody>(
        '/api/conversations/$conversationId/messages',
        data: jsonEncode(body),
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Content-Type': 'application/json',
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $_token',
          },
        ),
      );
    } on DioException catch (e) {
      yield ErrorEvent(e.message ?? 'Request failed');
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
  /// the run is unknown or 410 if its events were pruned.
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
  }) async* {
    final Response<ResponseBody> response;
    try {
      response = await _dio.get<ResponseBody>(
        '/api/conversations/$conversationId/runs/$runId/events/stream',
        queryParameters: {'since': since},
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $_token',
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

    final Response<ResponseBody> response;
    try {
      response = await _dio.post<ResponseBody>(
        '/api/conversations/$conversationId/voice',
        data: formData,
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $_token',
          },
        ),
      );
    } on DioException catch (e) {
      yield ErrorEvent(e.message ?? 'Voice upload failed');
      return;
    }

    yield* _parseSse(withHeartbeatTimeout(response.data!.stream));
  }

  /// Fetch the audio bytes for a message (GET /api/messages/{id}/audio).
  Future<({Uint8List bytes, String mimeType})?> fetchMessageAudio(
    String messageId,
  ) async {
    try {
      final response = await _dio.get<List<int>>(
        '/api/messages/$messageId/audio',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $_token'},
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
      final response = await _dio.get<List<int>>(
        '/api/audio/$audioId',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $_token'},
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
  }) {
    final base = _dio.options.baseUrl;
    final qp = <String, String>{
      'access_token': _token,
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
    return openConversationListStream(adapter: adapter);
  }

  Stream<ConversationListEvent> _streamConversationsViaDio({
    String? agentId,
  }) async* {
    final queryParams = <String, dynamic>{};
    if (agentId != null) queryParams['agent_id'] = agentId;

    final Response<ResponseBody> response;
    try {
      response = await _dio.get<ResponseBody>(
        '/api/conversations/stream',
        queryParameters: queryParams,
        options: Options(
          responseType: ResponseType.stream,
          headers: {
            'Accept': 'text/event-stream',
            'Authorization': 'Bearer $_token',
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

// -- Heartbeat timeout wrapper ------------------------------------------------

/// Wraps an SSE byte stream with a watchdog timer that fires a
/// [TimeoutException] if no data arrives within [timeout].
///
/// The server sends keepalive comments every 30 seconds. A 90-second default
/// tolerates 2 missed keepalives before declaring the connection stale.
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
        yield AgentErrorEvent(dataLine, sequenceId: seq);
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
