// app/lib/api/api_client.dart
//
// Wraps the generated assistant_api Dio client and adds SSE streaming support
// for the chat endpoint, which cannot be modelled by openapi-generator.

import 'dart:async';
import 'dart:convert';

import 'package:assistant_api/assistant_api.dart';
import 'package:dio/dio.dart';

import 'models/stream_event.dart';

/// Configured client bundle: generated API instances + SSE streaming helper.
class ApiClient {
  ApiClient({required String baseUrl, required String token})
      : _token = token {
    _dio = Dio(
      BaseOptions(
        baseUrl: baseUrl,
        connectTimeout: const Duration(seconds: 15),
        receiveTimeout: const Duration(minutes: 10),
      ),
    );

    _generatedApi = AssistantApi(
      dio: _dio,
      basePathOverride: baseUrl,
    );
    _generatedApi.setBearerAuth('bearer_token', token);
  }

  final String _token;
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
  final lines = byteStream
      .map(utf8.decode)
      .transform(const LineSplitter());

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
      }
      eventType = null;
      dataLine = null;
    }
  }
}
