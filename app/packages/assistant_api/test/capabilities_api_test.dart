import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for CapabilitiesApi
void main() {
  final instance = AssistantApi().getCapabilitiesApi();

  group(CapabilitiesApi, () {
    // `GET /api/capabilities` — return server capability flags.
    //
    //Future<ServerCapabilities> getCapabilities() async
    test('test getCapabilities', () async {
      // TODO
    });

  });
}
