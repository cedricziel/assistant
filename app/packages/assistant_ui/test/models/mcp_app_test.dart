import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('McpApp.fromJson', () {
    const base = {
      'server': 'weather',
      'resource_uri': 'ui://weather/forecast',
      'html': '<p>Sunny</p>',
    };

    test('reads prefers_border when it is a bool', () {
      final app = McpApp.fromJson({...base, 'prefers_border': true});
      expect(app?.prefersBorder, isTrue);
    });

    test('ignores a prefers_border that is not a bool', () {
      final app = McpApp.fromJson({...base, 'prefers_border': 'false'});
      expect(app, isNotNull);
      expect(app!.prefersBorder, isNull);
    });
  });
}
