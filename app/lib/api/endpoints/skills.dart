import 'dart:convert';

import '../client.dart';
import '../models/skill.dart';
import 'conversations.dart';

/// API endpoint client for skill discovery.
///
/// Wraps `GET /api/personas/{id}/skills`.
class SkillsEndpoint {
  const SkillsEndpoint(this._client);

  final AssistantClient _client;

  /// `GET /api/personas/{personaId}/skills` — list skills for a persona with
  /// their enabled/disabled state.
  ///
  /// Throws [NotFoundException] if the persona does not exist.
  Future<List<Skill>> listForPersona(String personaId) async {
    final response = await _client.get('/api/personas/$personaId/skills');
    if (response.statusCode == 404) {
      throw NotFoundException('Persona not found: $personaId');
    }
    if (response.statusCode != 200) {
      throw ApiException(response.statusCode, response.body);
    }
    final list = jsonDecode(response.body) as List<dynamic>;
    return list
        .cast<Map<String, dynamic>>()
        .map(Skill.fromJson)
        .toList();
  }
}
