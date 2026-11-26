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
    testWidgets('rebinds to a different tool call at the same position', (
      tester,
    ) async {
      await _pump(tester);
      const other = McpApp(
        server: 'weather',
        resourceUri: 'ui://weather/radar',
        html: '<p>Radar</p>',
      );
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: ToolCallRecord(
            toolName: 'mcp__weather__radar',
            toolCallId: 'call-2',
            status: ToolCallStatus.ok,
            app: other,
          ),
          conversationId: 'conv-1',
        ),
      );

      // The old surface is disposed once its teardown wait runs out.
      await tester.pump(const Duration(seconds: 1));

      expect(FakeMcpAppSurface.created, hasLength(2));
      expect(FakeMcpAppSurface.created.first.disposed, isTrue);
      expect(FakeMcpAppSurface.created.last.app.resourceUri, other.resourceUri);

      await _unmount(tester);
    });

    testWidgets('rebinds when the app or conversation changes under the '
        'same tool call', (tester) async {
      ToolCallRecord untracked(McpApp app) => ToolCallRecord(
        toolName: 'mcp__${app.server}__forecast',
        status: ToolCallStatus.ok,
        app: app,
      );
      await _pump(tester, child: McpAppFrame(toolCall: untracked(_app)));
      const otherServer = McpApp(
        server: 'climate',
        resourceUri: 'ui://weather/forecast',
        html: '<p>Cloudy</p>',
      );
      await _pump(tester, child: McpAppFrame(toolCall: untracked(otherServer)));
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: untracked(otherServer),
          conversationId: 'conv-2',
        ),
      );

      expect(FakeMcpAppSurface.created, hasLength(3));
      expect(FakeMcpAppSurface.created[1].app.server, 'climate');

      await _unmount(tester);
    });

    testWidgets('rebinds when the app policy or an id-less call changes', (
      tester,
    ) async {
      ToolCallRecord untracked(McpApp app, {String? result}) => ToolCallRecord(
        toolName: 'mcp__weather__forecast',
        status: ToolCallStatus.ok,
        result: result,
        app: app,
      );
      const narrowed = McpApp(
        server: 'weather',
        resourceUri: 'ui://weather/forecast',
        html: '<p>Sunny</p>',
        csp: McpAppCsp(connectDomains: ['api.weather.example']),
        permissions: McpAppPermissions(geolocation: true),
      );
      const sameButNarrowed = McpApp(
        server: 'weather',
        resourceUri: 'ui://weather/forecast',
        html: '<p>Sunny</p>',
        csp: McpAppCsp(connectDomains: ['api.weather.example']),
      );
      await _pump(tester, child: McpAppFrame(toolCall: untracked(narrowed)));
      // An equal policy in separate instances keeps the surface.
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: untracked(
            McpApp(
              server: 'weather',
              resourceUri: 'ui://weather/forecast',
              html: '<p>Sunny</p>',
              csp: McpAppCsp(connectDomains: ['api.weather.example'.trim()]),
              permissions: McpAppPermissions(geolocation: bool.parse('true')),
            ),
          ),
        ),
      );
      await _pump(
        tester,
        child: McpAppFrame(toolCall: untracked(sameButNarrowed)),
      );
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: untracked(sameButNarrowed, result: 'Rain'),
        ),
      );

      expect(FakeMcpAppSurface.created, hasLength(3));
      expect(FakeMcpAppSurface.created[1].app.permissions.geolocation, isFalse);

      await _unmount(tester);
    });

    testWidgets('rebinds on CSP lists that flatten alike and on an id-less '
        'status change', (tester) async {
      ToolCallRecord untracked(
        McpApp app, {
        ToolCallStatus status = ToolCallStatus.ok,
      }) => ToolCallRecord(
        toolName: 'mcp__weather__forecast',
        status: status,
        app: app,
      );
      McpApp withCsp(McpAppCsp csp) => McpApp(
        server: 'weather',
        resourceUri: 'ui://weather/forecast',
        html: '<p>Sunny</p>',
        csp: csp,
      );
      final wideConnect = withCsp(
        const McpAppCsp(connectDomains: ['|', 'https://b.example']),
      );
      final wideResource = withCsp(
        const McpAppCsp(resourceDomains: ['https://b.example', '|']),
      );
      await _pump(tester, child: McpAppFrame(toolCall: untracked(wideConnect)));
      await _pump(
        tester,
        child: McpAppFrame(toolCall: untracked(wideResource)),
      );
      await _pump(
        tester,
        child: McpAppFrame(
          toolCall: untracked(wideResource, status: ToolCallStatus.error),
        ),
      );

      expect(FakeMcpAppSurface.created, hasLength(3));

      await _unmount(tester);
    });

    testWidgets('sends app calls to the backend current at call time', (
      tester,
    ) async {
      await _pump(tester);
      final reconnected = await _pump(tester);

      await _appRequest(tester, 5, 'tools/call', {'name': 'forecast'});

      expect(reconnected.calls.map((c) => c.$1), ['forecast']);

      await _unmount(tester);
    });

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
