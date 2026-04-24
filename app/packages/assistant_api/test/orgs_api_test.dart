import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for OrgsApi
void main() {
  final instance = AssistantApi().getOrgsApi();

  group(OrgsApi, () {
    // `POST /api/orgs` — create a new organization.
    //
    //Future<OrgDetail> createOrg(CreateOrgRequest createOrgRequest) async
    test('test createOrg', () async {
      // TODO
    });

    // `GET /api/orgs/{id}` — get organization detail.
    //
    //Future<OrgDetail> getOrg(String id) async
    test('test getOrg', () async {
      // TODO
    });

    // `GET /api/orgs` — list organizations the user has access to.
    //
    //Future<BuiltList<OrgSummary>> listOrgs() async
    test('test listOrgs', () async {
      // TODO
    });

    // `PATCH /api/orgs/{id}` — update organization settings.
    //
    //Future<OrgDetail> updateOrg(String id, UpdateOrgRequest updateOrgRequest) async
    test('test updateOrg', () async {
      // TODO
    });
  });
}
