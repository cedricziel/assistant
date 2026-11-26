import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for McpAppView
void main() {
  final instance = McpAppViewBuilder();
  // TODO add properties to the builder and call build()

  group(McpAppView, () {
    // McpAppCspView csp
    test('to test the property `csp`', () async {
      // TODO
    });

    // The app document, served as `text/html;profile=mcp-app`.
    // String html
    test('to test the property `html`', () async {
      // TODO
    });

    // McpAppPermissionsView permissions
    test('to test the property `permissions`', () async {
      // TODO
    });

    // Whether the app asks to be drawn with a border.
    // bool prefersBorder
    test('to test the property `prefersBorder`', () async {
      // TODO
    });

    // The `ui://` resource the HTML was read from.
    // String resourceUri
    test('to test the property `resourceUri`', () async {
      // TODO
    });

    // Namespace of the server the tool came from, as in `mcp__{server}__{tool}`.
    // String server
    test('to test the property `server`', () async {
      // TODO
    });
  });
}
