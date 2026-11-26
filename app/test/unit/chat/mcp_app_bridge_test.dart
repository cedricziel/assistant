import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_bridge.dart';

import '../../support/fake_mcp_app.dart';

const _context = McpAppHostContext(
  theme: 'dark',
  locale: 'en-US',
  timeZone: 'UTC',
  platform: 'web',
);

ToolCallRecord _record({ToolCallStatus status = ToolCallStatus.ok}) =>
    ToolCallRecord(
      toolName: 'mcp__weather__forecast',
      toolCallId: 'call-1',
      status: status,
      arguments: {'city': 'Berlin'},
      result: 'Sunny',
      data: {'temperature': 21},
    );

class _Harness {
  _Harness({
    ToolCallRecord? record,
    this.confirmCalls = true,
    this.confirmLinks = true,
  }) {
    bridge = McpAppBridge(
      port: port,
      backend: backend,
      toolCall: record ?? _record(),
      hostContext: _context,
      confirmToolCall: (name, args) async {
        confirmedCalls.add(name);
        return confirmCalls;
      },
      confirmOpenLink: (url) async {
        confirmedLinks.add(url);
        return confirmLinks;
      },
      openLink: (url) async => openedLinks.add(url),
      onSizeChanged: heights.add,
    );
  }

  final port = FakeMcpAppPort();
  final backend = FakeMcpAppBackend();
  late final McpAppBridge bridge;
  bool confirmCalls;
  bool confirmLinks;
  final confirmedCalls = <String>[];
  final confirmedLinks = <Uri>[];
  final openedLinks = <Uri>[];
  final heights = <double>[];

  Future<void> initialize() async {
    port.request(1, 'ui/initialize', {
      'protocolVersion': '2026-01-26',
      'appInfo': {'name': 'weather', 'version': '1'},
      'appCapabilities': <String, Object?>{},
    });
    await pumpEventQueue();
    port.notify('ui/notifications/initialized');
    await pumpEventQueue();
  }

  Future<Map<String, Object?>> request(
    Object id,
    String method,
    Map<String, Object?> params,
  ) async {
    port.request(id, method, params);
    await pumpEventQueue();
    return port.replyTo(id);
  }

  List<String?> get sentMethods =>
      port.sent.map((m) => m['method'] as String?).toList();
}

void main() {
  group('McpAppBridge', () {
    test('answers ui/initialize with host info and context', () async {
      final h = _Harness();
      final reply = await h.request(1, 'ui/initialize', {
        'protocolVersion': '2026-01-26',
      });

      final result = reply['result']! as Map<String, Object?>;
      expect(reply['jsonrpc'], '2.0');
      expect(result['protocolVersion'], '2026-01-26');
      expect(result['hostInfo'], {'name': 'assistant', 'version': 'unknown'});
      expect(result['hostCapabilities'], isA<Map<String, Object?>>());
      expect(result['hostContext'], {
        'theme': 'dark',
        'displayMode': 'inline',
        'availableDisplayModes': ['inline'],
        'locale': 'en-US',
        'timeZone': 'UTC',
        'platform': 'web',
      });
    });

    test('sends nothing but the reply before the app is initialized', () async {
      final h = _Harness();
      await h.request(1, 'ui/initialize', {'protocolVersion': '2026-01-26'});

      expect(h.sentMethods, [null], reason: 'only the initialize reply');
    });

    test(
      'sends the tool input once, then the result, after initialized',
      () async {
        final h = _Harness();
        await h.initialize();
        h.port.notify('ui/notifications/initialized');
        await pumpEventQueue();

        final notifications = h.port.sent
            .where((m) => m['id'] == null)
            .toList();
        expect(notifications.map((m) => m['method']), [
          'ui/notifications/tool-input',
          'ui/notifications/tool-result',
        ]);
        expect(notifications[0]['params'], {
          'arguments': {'city': 'Berlin'},
        });
        expect(notifications[1]['params'], {
          'content': [
            {'type': 'text', 'text': 'Sunny'},
          ],
          'structuredContent': {'temperature': 21},
          'isError': false,
        });
      },
    );

    test('marks a failed call as an error result', () async {
      final h = _Harness(record: _record(status: ToolCallStatus.error));
      await h.initialize();

      final result = h.port.sent.last;
      expect(result['method'], 'ui/notifications/tool-result');
      expect((result['params']! as Map)['isError'], isTrue);
    });

    test(
      'sends tool-cancelled instead of a result for a cancelled call',
      () async {
        final h = _Harness(record: _record(status: ToolCallStatus.cancelled));
        await h.initialize();

        expect(h.sentMethods.whereType<String>(), [
          'ui/notifications/tool-input',
          'ui/notifications/tool-cancelled',
        ]);
      },
    );

    test('resizes the frame on size-changed', () async {
      final h = _Harness();
      h.port.notify('ui/notifications/size-changed', {
        'width': 300,
        'height': 240,
      });
      await pumpEventQueue();

      expect(h.heights, [240]);
    });

    test('opens a link only after the user confirms it', () async {
      final h = _Harness();
      final reply = await h.request(7, 'ui/open-link', {
        'url': 'https://example.com/docs',
      });

      expect(h.confirmedLinks, [Uri.parse('https://example.com/docs')]);
      expect(h.openedLinks, [Uri.parse('https://example.com/docs')]);
      expect(reply['result'], <String, Object?>{});
    });

    test('does not open a link the user declines', () async {
      final h = _Harness(confirmLinks: false);
      final reply = await h.request(7, 'ui/open-link', {
        'url': 'https://example.com/docs',
      });

      expect(h.openedLinks, isEmpty);
      expect((reply['error']! as Map)['code'], -32000);
    });

    test('proxies a read-only tool call without asking', () async {
      final h = _Harness();
      h.backend.toolResult = {
        'content': [
          {'type': 'text', 'text': '22°'},
        ],
      };
      final reply = await h.request(3, 'tools/call', {
        'name': 'refresh',
        'arguments': {'city': 'Berlin'},
      });

      final (name, arguments, confirmed) = h.backend.calls.single;
      expect(name, 'refresh');
      expect(arguments, {'city': 'Berlin'});
      expect(confirmed, isFalse);
      expect(h.confirmedCalls, isEmpty);
      expect(reply['result'], h.backend.toolResult);
    });

    test('asks before retrying a call that needs confirmation', () async {
      final h = _Harness();
      h.backend.refuseUnconfirmed = true;
      final reply = await h.request(3, 'tools/call', {
        'name': 'book',
        'arguments': <String, Object?>{},
      });

      expect(h.confirmedCalls, ['book']);
      expect(h.backend.calls.map((c) => c.$3), [false, true]);
      expect(reply['result'], isNotNull);
    });

    test('does not send a call the user declines', () async {
      final h = _Harness(confirmCalls: false);
      h.backend.refuseUnconfirmed = true;
      final reply = await h.request(3, 'tools/call', {'name': 'book'});

      expect(h.backend.calls.map((c) => c.$3), [false]);
      expect((reply['error']! as Map)['code'], -32000);
    });

    test(
      'rejects a call the server forbids, such as a cross-server tool',
      () async {
        final h = _Harness();
        h.backend.refusal = const McpAppRefused(403, "not the app's server");
        final reply = await h.request(3, 'tools/call', {'name': 'other'});

        final error = reply['error']! as Map;
        expect(error['code'], -32000);
        expect(error['message'], "not the app's server");
        expect(h.confirmedCalls, isEmpty);
      },
    );

    test('proxies resources/read', () async {
      final h = _Harness();
      final reply = await h.request(4, 'resources/read', {
        'uri': 'ui://weather/icon',
      });

      expect(h.backend.reads, ['ui://weather/icon']);
      expect(reply['result'], h.backend.resourceResult);
    });

    test('answers an unknown method with method-not-found', () async {
      final h = _Harness();
      final reply = await h.request(5, 'ui/request-display-mode', {});

      expect((reply['error']! as Map)['code'], -32601);
    });

    test('sends resource-teardown when closed', () async {
      final h = _Harness();
      await h.initialize();
      final closing = h.bridge.close();
      await pumpEventQueue();

      final teardown = h.port.sent.last;
      expect(teardown['method'], 'ui/resource-teardown');
      h.port.fromApp({
        'jsonrpc': '2.0',
        'id': teardown['id'],
        'result': <String, Object?>{},
      });
      await closing;
    });

    test('stops answering once closed', () async {
      final h = _Harness();
      await h.bridge.close(timeout: Duration.zero);
      final before = h.port.sent.length;
      h.port.request(9, 'ping');
      await pumpEventQueue();

      expect(h.port.sent.length, before);
    });
  });
}
