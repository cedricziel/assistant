import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for InterfacesApi
void main() {
  final instance = AssistantApi().getInterfacesApi();

  group(InterfacesApi, () {
    // `POST /api/orgs/{org_id}/spaces/{space_id}/interfaces` — create an interface instance.
    //
    //Future<InterfaceInstanceResponse> createInterface(String orgId, String spaceId, CreateInterfaceInstanceRequest createInterfaceInstanceRequest) async
    test('test createInterface', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/spaces/{space_id}/interfaces/{id}` — delete an interface instance.
    //
    //Future deleteInterface(String orgId, String spaceId, String id) async
    test('test deleteInterface', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces/{space_id}/interfaces` — list interface instances.
    //
    //Future<BuiltList<InterfaceInstanceResponse>> listInterfaces(String orgId, String spaceId) async
    test('test listInterfaces', () async {
      // TODO
    });
  });
}
