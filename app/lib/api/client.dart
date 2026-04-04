import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'models/stream_event.dart';

/// HTTP client for the assistant backend API.
///
/// Handles:
/// - Bearer token injection on every request.
/// - JSON response decoding.
/// - SSE stream parsing via [streamSse].
class AssistantClient {
  AssistantClient({required this.baseUrl, required this.token})
      : _client = http.Client();

  AssistantClient.withClient({
    required this.baseUrl,
    required this.token,
    required http.Client client,
  }) : _client = client;

  /// Base URL of the assistant server, e.g. `http://localhost:8080`.
  final String baseUrl;

  /// Bearer token for authentication.
  final String token;

  final http.Client _client;

  // -- Helpers ----------------------------------------------------------------

  Map<String, String> get _authHeaders => {
        'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      };

  Uri _uri(String path, [Map<String, String?>? queryParams]) {
    final base = Uri.parse(baseUrl);
    final uri = base.replace(
      path: path,
      queryParameters: queryParams != null
          ? Map.fromEntries(
              queryParams.entries
                  .where((e) => e.value != null)
                  .map((e) => MapEntry(e.key, e.value!)),
            )
          : null,
    );
    return uri;
  }

  // -- HTTP methods -----------------------------------------------------------

  /// Perform an authenticated GET request.
  Future<http.Response> get(
    String path, {
    Map<String, String?>? queryParams,
  }) async {
    return _client.get(
      _uri(path, queryParams),
      headers: _authHeaders,
    );
  }

  /// Perform an authenticated POST request with a JSON body.
  Future<http.Response> post(String path, {Object? body}) async {
    return _client.post(
      _uri(path),
      headers: _authHeaders,
      body: body != null ? jsonEncode(body) : null,
    );
  }

  /// Perform an authenticated DELETE request.
  Future<http.Response> delete(String path) async {
    return _client.delete(
      _uri(path),
      headers: _authHeaders,
    );
  }

  /// Perform an authenticated PATCH request with a JSON body.
  Future<http.Response> patch(String path, {Object? body}) async {
    return _client.patch(
      _uri(path),
      headers: _authHeaders,
      body: body != null ? jsonEncode(body) : null,
    );
  }

  // -- SSE streaming ----------------------------------------------------------

  /// Open an SSE stream to the given [path] with [body] as JSON, returning a
  /// [Stream] of [StreamEvent]s.
  ///
  /// The caller receives:
  /// - [TokenEvent] for each `event:token` line (incremental text).
  /// - [DoneEvent] when `event:done` arrives (final full response).
  /// - [ErrorEvent] on HTTP errors or unexpected stream closure.
  Stream<StreamEvent> streamSse(String path, Object body) {
    final controller = StreamController<StreamEvent>();

    () async {
      try {
        final request = http.Request('POST', _uri(path))
          ..headers.addAll({
            'Authorization': 'Bearer $token',
            'Content-Type': 'application/json',
            'Accept': 'text/event-stream',
          })
          ..body = jsonEncode(body);

        final response = await _client.send(request);

        if (response.statusCode != 200) {
          controller.add(
            ErrorEvent(
              'Server returned ${response.statusCode}',
            ),
          );
          await controller.close();
          return;
        }

        // Parse SSE lines from the response byte stream.
        await for (final event in _parseSse(response.stream)) {
          controller.add(event);
          if (event is DoneEvent || event is ErrorEvent) {
            break;
          }
        }
      } catch (e) {
        controller.add(ErrorEvent(e.toString()));
      } finally {
        await controller.close();
      }
    }();

    return controller.stream;
  }

  // -- SSE Parser -------------------------------------------------------------

  /// Parse an SSE byte stream into a [Stream] of [StreamEvent]s.
  ///
  /// Handles the wire format:
  /// ```
  /// event:token
  /// data: Hello
  ///
  /// event:done
  /// data: {"role":"assistant","content":"Hello"}
  /// ```
  Stream<StreamEvent> _parseSse(Stream<List<int>> byteStream) async* {
    final lineBuffer = StringBuffer();
    String? currentEvent;

    await for (final chunk in byteStream
        .transform(utf8.decoder)
        .transform(const LineSplitter())) {
      final line = chunk;

      if (line.isEmpty) {
        // Empty line = dispatch current event.
        // Reset for next event.
        currentEvent = null;
        continue;
      }

      if (line.startsWith('event:')) {
        currentEvent = line.substring(6).trim();
        lineBuffer.clear();
        continue;
      }

      if (line.startsWith('data:')) {
        final data = line.substring(5);
        // data: may have a leading space
        final trimmedData = data.startsWith(' ') ? data.substring(1) : data;

        if (currentEvent == 'token') {
          yield TokenEvent(trimmedData);
        } else if (currentEvent == 'done') {
          try {
            final json = jsonDecode(trimmedData) as Map<String, dynamic>;
            yield DoneEvent.fromJson(json);
          } catch (_) {
            yield DoneEvent(role: 'assistant', content: trimmedData);
          }
          return;
        }
        // Ignore unknown event types silently.
        continue;
      }
    }
  }

  /// Close the underlying HTTP client.
  void dispose() {
    _client.close();
  }
}
