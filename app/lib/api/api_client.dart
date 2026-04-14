// app/lib/api/api_client.dart
//
// Wraps the generated assistant_api Dio client and adds SSE streaming support
// for the chat endpoint, which cannot be modelled by openapi-generator.

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:dio/dio.dart';

import 'models/server_capabilities.dart';
import 'models/stream_event.dart';

/// Configured client bundle: generated API instances + SSE streaming helper.
class ApiClient {
  ApiClient({required String baseUrl, required String token})
    : _token = token,
      _baseUrl = baseUrl {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 15),
        receiveTimeout: const Duration(minutes: 10),
      ),
    );

    _generatedApi = AssistantApi(dio: _dio, basePathOverride: baseUrl);
    _generatedApi.setBearerAuth('bearer_token', token);
  }

  final String _token;
  final String _baseUrl;
  late final Dio _dio;
  late final AssistantApi _generatedApi;

  ConversationsApi get conversations => _generatedApi.getConversationsApi();
  PersonasApi get personas => _generatedApi.getPersonasApi();
  SkillsApi get skills => _generatedApi.getSkillsApi();
  LogsApi get logs => _generatedApi.getLogsApi();
  TracesApi get traces => _generatedApi.getTracesApi();
  WebhooksApi get webhooks => _generatedApi.getWebhooksApi();
  AgentsApi get agents => _generatedApi.getAgentsApi();
  AnalyticsApi get analytics => _generatedApi.getAnalyticsApi();
  WorkflowsApi get workflows => _generatedApi.getWorkflowsApi();

  /// Stream assistant tokens for a conversation message.
  ///
  /// Yields [TokenEvent] for each incremental chunk and a final [DoneEvent]
  /// when the server closes the stream.
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message,
  ) async* {
    final Response<ResponseBody> response;
    try {
      response = await _dio.post<ResponseBody>(
        '/api/conversations/$conversationId/messages',
        data: jsonEncode({'message': message}),
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

    yield* _parseSse(response.data!.stream);
  }

  // -- Voice / capabilities ---------------------------------------------------

  /// Fetch server capability flags (voice_send, voice_receive, …).
  ///
  /// Uses dart:io HttpClient directly (no Dio interceptor chain) so no
  /// zero-duration dart:async Timer is created during the request setup,
  /// which keeps Flutter's fakeAsync tests clean.
  Future<ServerCapabilities> getCapabilities({CancelToken? cancelToken}) async {
    final client = HttpClient();
    // Disable connect timeout — no dart:async Timer created.
    client.connectionTimeout = null;
    try {
      final uri = Uri.parse('$_baseUrl/api/capabilities');
      final request = await client.getUrl(uri);
      request.headers.set('Authorization', 'Bearer $_token');
      if (cancelToken?.isCancelled == true) return ServerCapabilities.disabled;
      final response = await request.close();
      if (response.statusCode == 200) {
        final body = await response.transform(utf8.decoder).join();
        final json = jsonDecode(body) as Map<String, dynamic>;
        return ServerCapabilities.fromJson(json);
      }
    } catch (_) {
      // Network, cancellation, or parse errors → report no capabilities.
    } finally {
      client.close(force: true);
    }
    return ServerCapabilities.disabled;
  }

  /// Upload [audioBytes] to the voice endpoint and stream the SSE response.
  Stream<StreamEvent> sendVoiceMessage(
    String conversationId,
    Uint8List audioBytes,
    String mimeType,
  ) async* {
    final extension = mimeType.contains('webm') ? 'webm' : 'm4a';
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

    yield* _parseSse(response.data!.stream);
  }

  /// Fetch the audio bytes for a message (GET /api/messages/{id}/audio).
  Future<Uint8List?> fetchMessageAudio(String messageId) async {
    try {
      final response = await _dio.get<List<int>>(
        '/api/messages/$messageId/audio',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $_token'},
        ),
      );
      if (response.statusCode == 200 && response.data != null) {
        return Uint8List.fromList(response.data!);
      }
    } catch (_) {
      // ignore
    }
    return null;
  }

  /// Fetch audio bytes by audio ID (GET /api/audio/{id}).
  Future<Uint8List?> fetchAudio(String audioId) async {
    try {
      final response = await _dio.get<List<int>>(
        '/api/audio/$audioId',
        options: Options(
          responseType: ResponseType.bytes,
          headers: {'Authorization': 'Bearer $_token'},
        ),
      );
      if (response.statusCode == 200 && response.data != null) {
        return Uint8List.fromList(response.data!);
      }
    } catch (_) {
      // ignore
    }
    return null;
  }

  // -- SSE parser -------------------------------------------------------------

  Stream<StreamEvent> _parseSse(Stream<List<int>> byteStream) =>
      parseSseByteStream(byteStream);
}

// -- SSE parser (top-level for testability) -----------------------------------

/// Parses a raw SSE byte stream into [StreamEvent]s.
///
/// Accepts [Stream<List<int>>] or [Stream<Uint8List>] (both are [List<int>]).
/// Uses [utf8.decode] via [Stream.map] instead of [utf8.decoder] to avoid a
/// runtime type error on iOS where [Utf8Decoder] is not accepted as a
/// [StreamTransformer<Uint8List, String>].
Stream<StreamEvent> parseSseByteStream(Stream<List<int>> byteStream) async* {
  final lines = byteStream.map(utf8.decode).transform(const LineSplitter());

  String? eventType;
  String? dataLine;

  await for (final line in lines) {
    if (line.startsWith('event:')) {
      eventType = line.substring('event:'.length).trim();
    } else if (line.startsWith('data:')) {
      dataLine = line.substring('data:'.length).trim();
    } else if (line.isEmpty) {
      // Blank line — dispatch the event.
      if (eventType == 'token' && dataLine != null) {
        yield TokenEvent(dataLine);
      } else if (eventType == 'status' && dataLine != null) {
        yield StatusEvent(dataLine);
      } else if (eventType == 'tool_result' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield ToolResultEvent.fromJson(json);
        } catch (_) {
          // ignore malformed tool_result events
        }
      } else if (eventType == 'done' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield DoneEvent(
            role: json['role'] as String? ?? 'assistant',
            content: json['content'] as String? ?? '',
          );
        } catch (_) {
          yield DoneEvent(role: 'assistant', content: dataLine);
        }
      } else if (eventType == 'audio_ready' && dataLine != null) {
        try {
          final json = jsonDecode(dataLine) as Map<String, dynamic>;
          yield AudioReadyEvent.fromJson(json);
        } catch (_) {
          // ignore malformed audio_ready events
        }
      }
      eventType = null;
      dataLine = null;
    }
  }
}
