import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for SpacesApi
void main() {
  final instance = AssistantApi().getSpacesApi();

  group(SpacesApi, () {
    // `POST /api/orgs/{org_id}/spaces` — create a space (org-admin only).
    //
    //Future<SpaceDetail> createSpace(String orgId, CreateSpaceRequest createSpaceRequest) async
    test('test createSpace', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/spaces/{id}` — delete a space (org-admin only).
    //
    //Future deleteSpace(String orgId, String id) async
    test('test deleteSpace', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces/{id}` — get space detail.
    //
    //Future<SpaceDetail> getSpace(String orgId, String id) async
    test('test getSpace', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces` — list spaces filtered by membership.
    //
    //Future<BuiltList<SpaceSummary>> listSpaces(String orgId) async
    test('test listSpaces', () async {
      // TODO
    });

    // `PATCH /api/orgs/{org_id}/spaces/{id}` — update space name.
    //
    //Future<SpaceDetail> updateSpace(String orgId, String id, UpdateSpaceRequest updateSpaceRequest) async
    test('test updateSpace', () async {
      // TODO
    });
  });
}
