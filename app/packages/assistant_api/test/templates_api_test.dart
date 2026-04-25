import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for TemplatesApi
void main() {
  final instance = AssistantApi().getTemplatesApi();

  group(TemplatesApi, () {
    // `POST /api/orgs/{org_id}/spaces/{space_id}/personas/from-template` — create a persona from a template.
    //
    //Future<PersonaFromTemplateResponse> createFromTemplate(String orgId, String spaceId, CreateFromTemplateRequest createFromTemplateRequest) async
    test('test createFromTemplate', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/catalog/templates` — list available persona templates.
    //
    //Future<BuiltList<TemplateResponse>> listTemplates(String orgId) async
    test('test listTemplates', () async {
      // TODO
    });

    // `GET /api/users/me/onboarding-status` — check if user has created at least one persona.
    //
    //Future<OnboardingStatusResponse> onboardingStatus() async
    test('test onboardingStatus', () async {
      // TODO
    });
  });
}
