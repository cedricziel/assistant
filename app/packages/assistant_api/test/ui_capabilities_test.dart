import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for UiCapabilities
void main() {
  final instance = UiCapabilitiesBuilder();
  // TODO add properties to the builder and call build()

  group(UiCapabilities, () {
    // Lower-case hosts whose `openUrl` targets open without a confirmation prompt. Empty means every link asks first.
    // BuiltList<String> linkHosts
    test('to test the property `linkHosts`', () async {
      // TODO
    });
  });
}
