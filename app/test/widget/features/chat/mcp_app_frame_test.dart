import 'package:assistant_ui/models.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_bridge.dart';
import 'package:assistant_app/features/chat/mcp_app_frame.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_provider.dart';

import '../../../support/fake_mcp_app.dart';

const _app = McpApp(
  server: 'weather',
  resourceUri: 'ui://weather/forecast',
  html: '<p>Sunny</p>',
);

ToolCallRecord _record() => ToolCallRecord(
  toolName: 'mcp__weather__forecast',
  toolCallId: 'call-1',
  status: ToolCallStatus.ok,
  arguments: {'city': 'Berlin'},
  result: 'Sunny',
  app: _app,
);

Future<FakeMcpAppBackend> _pump(WidgetTester tester, {Widget? child}) async {
  final backend = FakeMcpAppBackend();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        mcpAppSurfaceFactoryProvider.overrideWithValue(
          FakeMcpAppSurface.factory,
        ),
        mcpAppBackendProvider.overrideWith((ref, _) => backend),
      ],
      child: MaterialApp(
        home: Scaffold(
          body:
              child ??
              McpAppFrame(toolCall: _record(), conversationId: 'conv-1'),
        ),
      ),
    ),
  );
  return backend;
}

Future<Map<String, Object?>> _appRequest(
  WidgetTester tester,
  Object id,
  String method,
  Map<String, Object?> params,
) async {
  final port = FakeMcpAppSurface.created.last.port;
  port.request(id, method, params);
  await tester.pumpAndSettle();
  return port.replyTo(id);
}

/// Removes the frame and lets its teardown wait run out.
Future<void> _unmount(WidgetTester tester) async {
  await tester.pumpWidget(const SizedBox());
  await tester.pump(const Duration(seconds: 1));
}

void main() {
  setUp(FakeMcpAppSurface.created.clear);

  group('McpAppFrame', () {
    testWidgets('renders the app labelled as third-party from its server', (
      tester,
    ) async {
      await _pump(tester);

      expect(find.byKey(fakeMcpAppViewKey), findsOneWidget);
      expect(find.text('Third-party app from weather'), findsOneWidget);

      await _unmount(tester);
    });

    testWidgets('renders nothing for a tool call without an app', (
      tester,
    ) async {
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: ToolCallRecord(toolName: 'bash', status: ToolCallStatus.ok),
        ),
      );

      expect(find.byKey(fakeMcpAppViewKey), findsNothing);
      expect(FakeMcpAppSurface.created, isEmpty);
    });

    testWidgets('grows to the height the app reports', (tester) async {
      await _pump(tester);
      FakeMcpAppSurface.created.last.port.notify(
        'ui/notifications/size-changed',
        {'height': 320},
      );
      await tester.pump();

      expect(tester.getSize(find.byKey(fakeMcpAppViewKey)).height, 320);

      await _unmount(tester);
    });

    testWidgets('asks the user before a call the server will not run '
        'unconfirmed', (tester) async {
      final backend = await _pump(tester);
      backend.refuseUnconfirmed = true;
      final port = FakeMcpAppSurface.created.last.port;
      port.request(3, 'tools/call', {'name': 'book'});
      await tester.pumpAndSettle();

      expect(find.text('Allow weather to run "book"?'), findsOneWidget);
      expect(backend.calls.map((c) => c.$3), [false]);

      await tester.tap(find.text('Run'));
      await tester.pumpAndSettle();

      expect(backend.calls.map((c) => c.$3), [false, true]);
      expect(port.replyTo(3)['result'], isNotNull);

      await _unmount(tester);
    });

    testWidgets('does not send the call when the user cancels', (tester) async {
      final backend = await _pump(tester);
      backend.refuseUnconfirmed = true;
      final port = FakeMcpAppSurface.created.last.port;
      port.request(3, 'tools/call', {'name': 'book'});
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(backend.calls.map((c) => c.$3), [false]);
      final error = port.replyTo(3)['error']! as Map;
      expect(error['code'], -32000);

      await _unmount(tester);
    });

    testWidgets('rejects a call the server forbids without asking', (
      tester,
    ) async {
      final backend = await _pump(tester);
      backend.refusal = const McpAppRefused(403, "not the app's server");

      final reply = await _appRequest(tester, 4, 'tools/call', {
        'name': 'other',
      });

      expect(find.byType(AlertDialog), findsNothing);
      expect(reply['error'], {
        'code': -32000,
        'message': "not the app's server",
      });

      await _unmount(tester);
    });

    testWidgets('asks before opening a link the app requests', (tester) async {
      await _pump(tester);
      FakeMcpAppSurface.created.last.port.request(5, 'ui/open-link', {
        'url': 'https://example.com/docs',
      });
      await tester.pumpAndSettle();

      expect(find.text('Open link?'), findsOneWidget);
      expect(find.textContaining('https://example.com/docs'), findsOneWidget);

      await _unmount(tester);
    });

    testWidgets('tears the app down before disposing its view', (tester) async {
      await _pump(tester);
      final surface = FakeMcpAppSurface.created.last;

      await _pump(tester, child: const SizedBox());
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();

      expect(surface.port.sent.last['method'], 'ui/resource-teardown');
      expect(surface.disposed, isTrue);
    });
  });
}
