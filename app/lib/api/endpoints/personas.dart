import 'dart:convert';

import '../client.dart';
import '../models/persona.dart';
import 'conversations.dart';

/// API endpoint client for persona management.
///
/// Wraps `GET /api/personas` and `POST /api/personas/active`.
class PersonasEndpoint {
  const PersonasEndpoint(this._client);

  final AssistantClient _client;

  /// `GET /api/personas` — list all personas defined on the server.
  Future<List<Persona>> list() async {
    final response = await _client.get('/api/personas');
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    final list = jsonDecode(response.body) as List<dynamic>;
    return list
        .cast<Map<String, dynamic>>()
        .map(Persona.fromJson)
        .toList();
  }

  /// `POST /api/personas/active` — switch the active persona for the session.
  ///
  /// Throws [NotFoundException] if the persona ID does not exist.
  Future<Persona> setActive(String id) async {
    final response = await _client.post(
      '/api/personas/active',
      body: {'id': id},
    );
    if (response.statusCode == 404) {
      throw NotFoundException('Persona not found: $id');
    }
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    return Persona.fromJson(
      jsonDecode(response.body) as Map<String, dynamic>,
    );
  }
}
