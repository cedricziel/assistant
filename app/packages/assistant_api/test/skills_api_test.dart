import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for SkillsApi
void main() {
  final instance = AssistantApi().getSkillsApi();

  group(SkillsApi, () {
    // `GET /api/personas/{persona_id}/skills` — list skills for a persona.
    //
    //Future<BuiltList<SkillEntryResponse>> listPersonaSkills(String personaId) async
    test('test listPersonaSkills', () async {
      // TODO
    });

  });
}
