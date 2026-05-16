import 'package:assistant_api/assistant_api.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/traces/tool_call_span_card.dart';

SpanEntryResponse _toolSpan({
  required String tool,
  required String status,
  String? params,
  String? observation,
  String? error,
  int durationMs = 12,
}) {
  final attrs = <String, Object?>{
    'tool_name': tool,
    'tool_status': status,
    'tool_params': ?params,
    'tool_observation': ?observation,
    'tool_error': ?error,
    'iteration': 1,
    'turn': 2,
    'interface': 'Cli',
  };
  return SpanEntryResponse((b) {
    b
      ..name = 'execute_tool $tool'
      ..spanId = '$tool-sid'
      ..durationMs = durationMs
      ..startTime = DateTime(2026, 1, 1)
      ..attributes = JsonObject(attrs);
  });
}

Widget _wrap(Widget child, {Brightness brightness = Brightness.light}) {
  return MaterialApp(
    theme: ThemeData(
      colorScheme: ColorScheme.fromSeed(
        seedColor: Colors.blue,
        brightness: brightness,
      ),
      useMaterial3: true,
    ),
    home: Scaffold(body: SingleChildScrollView(child: child)),
  );
}

void main() {
  group('ToolCallSpanCard', () {
    testWidgets('ok status shows checkmark icon and ok pill', (tester) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'file-read',
              status: 'ok',
              params: '{"path":"/tmp/foo.txt"}',
              observation: 'ok, 42 bytes',
            ),
            totalMs: 50,
          ),
        ),
      );

      expect(find.text('file-read'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle), findsOneWidget);
      expect(find.text('ok'), findsOneWidget);
    });

    testWidgets('error status shows error icon and red pill', (tester) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'bash',
              status: 'error',
              error: 'command failed',
            ),
            totalMs: 50,
          ),
        ),
      );

      expect(find.byIcon(Icons.error), findsOneWidget);
      expect(find.text('error'), findsOneWidget);
    });

    testWidgets('denied status shows block icon', (tester) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'web-fetch',
              status: 'denied',
              error: 'user denied',
            ),
            totalMs: 50,
          ),
        ),
      );

      expect(find.byIcon(Icons.block), findsOneWidget);
      expect(find.text('denied'), findsOneWidget);
    });

    testWidgets('missing status falls back to unknown', (tester) async {
      final span = SpanEntryResponse((b) {
        b
          ..name = 'execute_tool legacy'
          ..spanId = 'legacy-sid'
          ..durationMs = 7
          ..startTime = DateTime(2026, 1, 1)
          ..attributes = JsonObject({'tool_name': 'legacy'});
      });
      await tester.pumpWidget(_wrap(ToolCallSpanCard(span: span, totalMs: 10)));

      expect(find.byIcon(Icons.help_outline), findsOneWidget);
      expect(find.text('unknown'), findsOneWidget);
    });

    testWidgets('expanding ok card reveals pretty-printed params + output', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'file-read',
              status: 'ok',
              params: '{"path":"/tmp/foo.txt"}',
              observation: 'ok, 42 bytes',
            ),
            totalMs: 50,
          ),
        ),
      );

      await tester.tap(find.text('file-read'));
      await tester.pumpAndSettle();

      expect(find.text('Params'), findsOneWidget);
      expect(find.text('Output'), findsOneWidget);
      // Pretty-printed JSON includes the key on its own line.
      expect(find.textContaining('"path"'), findsOneWidget);
      expect(find.textContaining('ok, 42 bytes'), findsOneWidget);
    });

    testWidgets('expanding error card shows error text', (tester) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'bash',
              status: 'error',
              error: 'permission denied',
            ),
            totalMs: 50,
          ),
        ),
      );

      await tester.tap(find.text('bash'));
      await tester.pumpAndSettle();

      expect(find.textContaining('permission denied'), findsOneWidget);
    });

    testWidgets('Show all attributes reveals iteration/turn/interface', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'file-read',
              status: 'ok',
              params: '{}',
              observation: 'done',
            ),
            totalMs: 50,
          ),
        ),
      );

      await tester.tap(find.text('file-read'));
      await tester.pumpAndSettle();

      // iteration/turn/interface should not be visible by default.
      expect(find.text('iteration'), findsNothing);

      await tester.tap(find.text('Show all attributes'));
      await tester.pumpAndSettle();

      expect(find.text('iteration'), findsOneWidget);
      expect(find.text('turn'), findsOneWidget);
      expect(find.text('interface'), findsOneWidget);
    });

    testWidgets('non-JSON params shown verbatim', (tester) async {
      await tester.pumpWidget(
        _wrap(
          ToolCallSpanCard(
            span: _toolSpan(
              tool: 'broken',
              status: 'ok',
              params: 'not json {{',
              observation: 'ok',
            ),
            totalMs: 50,
          ),
        ),
      );

      await tester.tap(find.text('broken'));
      await tester.pumpAndSettle();

      expect(find.text('not json {{'), findsOneWidget);
    });
  });
}
