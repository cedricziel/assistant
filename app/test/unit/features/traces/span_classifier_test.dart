import 'package:assistant_api/assistant_api.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/traces/span_classifier.dart';

SpanEntryResponse _span({
  required String name,
  Map<String, Object?>? attributes,
}) {
  return SpanEntryResponse((b) {
    b
      ..name = name
      ..spanId = 'sid'
      ..durationMs = 1
      ..startTime = DateTime(2026, 1, 1);
    if (attributes != null) {
      b.attributes = JsonObject(attributes);
    }
  });
}

void main() {
  group('isToolSpan', () {
    test('detects span name prefix execute_tool', () {
      expect(isToolSpan(_span(name: 'execute_tool file-read')), isTrue);
    });

    test('detects tool_name attribute when prefix is missing', () {
      expect(
        isToolSpan(
          _span(name: 'tool_invocation', attributes: {'tool_name': 'bash'}),
        ),
        isTrue,
      );
    });

    test('rejects generic span', () {
      expect(isToolSpan(_span(name: 'chat anthropic/claude')), isFalse);
    });

    test('rejects span with no name match and no tool_name attribute', () {
      expect(isToolSpan(_span(name: 'orchestrator')), isFalse);
    });
  });

  group('toolNameFromSpan', () {
    test('reads attribute first', () {
      expect(
        toolNameFromSpan(
          _span(
            name: 'execute_tool web-fetch',
            attributes: {'tool_name': 'web-fetch'},
          ),
        ),
        'web-fetch',
      );
    });

    test('falls back to suffix of execute_tool prefix', () {
      expect(toolNameFromSpan(_span(name: 'execute_tool bash')), 'bash');
    });

    test('returns empty when prefix missing and attribute absent', () {
      expect(toolNameFromSpan(_span(name: 'orchestrator')), '');
    });
  });
}
