import 'dart:convert';

import '../client.dart';
import '../models/conversation.dart';
import '../models/stream_event.dart';

/// API endpoint client for conversation management.
///
/// Wraps `GET/POST/DELETE/PATCH /api/conversations` and the SSE streaming
/// `POST /api/conversations/{id}/messages` endpoint.
class ConversationsEndpoint {
  const ConversationsEndpoint(this._client);

  final AssistantClient _client;

  // -- List conversations -----------------------------------------------------

  /// `GET /api/conversations` — list all conversations, newest first.
  Future<List<ConversationSummary>> list() async {
    final response = await _client.get('/api/conversations');
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    final list = jsonDecode(response.body) as List<dynamic>;
    return list
        .cast<Map<String, dynamic>>()
        .map(ConversationSummary.fromJson)
        .toList();
  }

  // -- Create conversation ----------------------------------------------------

  /// `POST /api/conversations` — create a new conversation.
  Future<ConversationSummary> create({String? title}) async {
    final response = await _client.post(
      '/api/conversations',
      body: {'title': title ?? 'New Chat'},
    );
    if (response.statusCode != 201) {
      throw ApiException(response.statusCode, response.body);
    }
    return ConversationSummary.fromJson(
      jsonDecode(response.body) as Map<String, dynamic>,
    );
  }

  // -- Get conversation -------------------------------------------------------

  /// `GET /api/conversations/{id}` — fetch a conversation with message history.
  Future<ConversationDetail> get(String id) async {
    final response = await _client.get('/api/conversations/$id');
    if (response.statusCode == 404) {
      throw NotFoundException('Conversation not found: $id');
    }
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    return ConversationDetail.fromJson(
      jsonDecode(response.body) as Map<String, dynamic>,
    );
  }

  // -- Delete conversation ----------------------------------------------------

  /// `DELETE /api/conversations/{id}` — delete a conversation and its messages.
  Future<void> delete(String id) async {
    final response = await _client.delete('/api/conversations/$id');
    if (response.statusCode != 204) {
      throw ApiException(response.statusCode, response.body);
    }
  }

  // -- Rename conversation ----------------------------------------------------

  /// `PATCH /api/conversations/{id}` — update a conversation's title.
  Future<ConversationSummary> rename(String id, String title) async {
    final response = await _client.patch(
      '/api/conversations/$id',
      body: {'title': title},
    );
    if (response.statusCode == 404) {
      throw NotFoundException('Conversation not found: $id');
    }
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    return ConversationSummary.fromJson(
      jsonDecode(response.body) as Map<String, dynamic>,
    );
  }

  // -- Send message (streaming) -----------------------------------------------

  /// `POST /api/conversations/{id}/messages` — send a user message and receive
  /// a streaming SSE response.
  ///
  /// Returns a [Stream] of [StreamEvent]s.  The caller should:
  /// 1. Accumulate [TokenEvent]s into a display buffer.
  /// 2. On [DoneEvent], replace the buffer with the canonical [DoneEvent.content].
  /// 3. On [ErrorEvent], show the error and stop.
  Stream<StreamEvent> sendMessage(String id, String message) {
    return _client.streamSse(
      '/api/conversations/$id/messages',
      {'message': message},
    );
  }
}

// -- Exceptions ---------------------------------------------------------------

/// Thrown when an API call returns a non-success HTTP status code.
class ApiException implements Exception {
  const ApiException(this.statusCode, this.body);

  final int statusCode;
  final String body;

  @override
  String toString() => 'ApiException($statusCode): $body';
}

/// Thrown when a resource is not found (HTTP 404).
class NotFoundException extends ApiException {
  const NotFoundException(String message) : super(404, message);
}

/// Thrown when authentication fails (HTTP 401).
class UnauthorizedException extends ApiException {
  const UnauthorizedException([String body = 'Unauthorized'])
      : super(401, body);
}
