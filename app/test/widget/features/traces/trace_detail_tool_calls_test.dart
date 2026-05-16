import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/traces/trace_detail_screen.dart';
import 'package:assistant_app/features/traces/tool_call_span_card.dart';
import 'package:assistant_app/features/traces/traces_provider.dart';

SpanEntryResponse _toolSpan({
  required String tool,
  required String status,
  Map<String, Object?> extra = const {},
  int durationMs = 12,
}) {
  final attrs = <String, Object?>{
    'tool_name': tool,
    'tool_status': status,
    ...extra,
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

SpanEntryResponse _chatSpan() {
  return SpanEntryResponse((b) {
    b
      ..name = 'chat anthropic/claude'
      ..spanId = 'chat-sid'
      ..durationMs = 200
      ..startTime = DateTime(2026, 1, 1, 0, 0, 1)
      ..attributes = JsonObject({'gen_ai.provider.name': 'anthropic'});
  });
}

TraceDetailResponse _detailWithSpans(List<SpanEntryResponse> spans) {
  return TraceDetailResponse((b) {
    b
      ..traceId = 'trace-1'
      ..personaId = 'persona-1'
      ..startTime = DateTime(2026, 1, 1)
      ..durationMs = 300
      ..spans.replace(BuiltList<SpanEntryResponse>(spans));
  });
}

Widget _harness(TraceDetailResponse detail) {
  final router = GoRouter(
    initialLocation: '/traces/trace-1',
    routes: [
      GoRoute(
        path: '/traces/:id',
        builder: (_, state) =>
            TraceDetailScreen(traceId: state.pathParameters['id']!),
      ),
      GoRoute(
        path: '/traces',
        builder: (_, _) => const Scaffold(body: Text('Traces')),
      ),
    ],
  );
  return ProviderScope(
    overrides: [
      traceDetailProvider(
        'trace-1',
      ).overrideWith(() => _StubNotifier(TraceDetailState(detail: detail))),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}

class _StubNotifier extends TraceDetailNotifier {
  _StubNotifier(this._state) : super('trace-1');
  final TraceDetailState _state;

  @override
  Future<TraceDetailState> build() async => _state;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'trace detail renders tool spans with dedicated cards and keeps generic span',
    (tester) async {
      final detail = _detailWithSpans([
        _toolSpan(
          tool: 'file-read',
          status: 'ok',
          extra: {
            'tool_params': '{"path":"/tmp/foo.txt"}',
            'tool_observation': 'ok, 42 bytes',
          },
        ),
        _toolSpan(
          tool: 'bash',
          status: 'error',
          extra: {'tool_error': 'permission denied'},
        ),
        _toolSpan(
          tool: 'web-fetch',
          status: 'denied',
          extra: {'tool_error': 'user denied'},
        ),
        _chatSpan(),
      ]);

      await tester.pumpWidget(_harness(detail));
      await tester.pumpAndSettle();

      expect(find.byType(ToolCallSpanCard), findsNWidgets(3));

      expect(find.text('file-read'), findsOneWidget);
      expect(find.text('bash'), findsOneWidget);
      expect(find.text('web-fetch'), findsOneWidget);

      // Status pills visible without expanding.
      expect(find.byIcon(Icons.check_circle), findsOneWidget);
      expect(find.byIcon(Icons.error), findsOneWidget);
      expect(find.byIcon(Icons.block), findsOneWidget);

      // Generic chat span still rendered (not as a ToolCallSpanCard).
      expect(find.text('chat anthropic/claude'), findsOneWidget);
    },
  );
}
