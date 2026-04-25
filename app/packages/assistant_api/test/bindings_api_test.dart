import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for BindingsApi
void main() {
  final instance = AssistantApi().getBindingsApi();

  group(BindingsApi, () {
    // `POST /api/orgs/{org_id}/spaces/{space_id}/bindings` — bind a persona to an interface.
    //
    //Future<BindingResponse> createBinding(String orgId, String spaceId, CreateBindingRequest createBindingRequest) async
    test('test createBinding', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/spaces/{space_id}/bindings/{id}` — delete a binding.
    //
    //Future deleteBinding(String orgId, String spaceId, String id) async
    test('test deleteBinding', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces/{space_id}/bindings` — list bindings.
    //
    //Future<BuiltList<BindingResponse>> listBindings(String orgId, String spaceId) async
    test('test listBindings', () async {
      // TODO
    });
  });
}
