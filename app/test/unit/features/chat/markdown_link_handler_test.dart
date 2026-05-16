import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:assistant_app/features/chat/markdown_link_handler.dart';

class _RecordingLauncher {
  final List<({Uri uri, LaunchMode mode})> calls = [];
  bool result = true;
  Object? throws;

  Future<bool> call(Uri uri, {LaunchMode mode = LaunchMode.platformDefault}) {
    calls.add((uri: uri, mode: mode));
    if (throws != null) {
      return Future.error(throws!);
    }
    return Future.value(result);
  }
}

Future<void> _pump(WidgetTester tester, void Function(BuildContext) onBuild) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) {
            onBuild(context);
            return const SizedBox.shrink();
          },
        ),
      ),
    ),
  );
}

void main() {
  group('MarkdownLinkHandler', () {
    testWidgets('launches allowed https scheme', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('https://example.com');

      expect(launcher.calls, hasLength(1));
      expect(launcher.calls.single.uri, Uri.parse('https://example.com'));
      expect(launcher.calls.single.mode, LaunchMode.externalApplication);
    });

    testWidgets('launches allowed http scheme', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('http://example.com');
      expect(launcher.calls, hasLength(1));
    });

    testWidgets('launches mailto scheme', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('mailto:hi@example.com');

      expect(launcher.calls, hasLength(1));
      expect(launcher.calls.single.uri, Uri.parse('mailto:hi@example.com'));
    });

    testWidgets('rejects javascript scheme without launching', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('javascript:alert(1)');
      await tester.pump();

      expect(launcher.calls, isEmpty);
      expect(find.textContaining('Cannot open link'), findsOneWidget);
    });

    testWidgets('rejects file scheme without launching', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('file:///etc/passwd');
      await tester.pump();

      expect(launcher.calls, isEmpty);
    });

    testWidgets('rejects data scheme without launching', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('data:text/html,<script>alert(1)</script>');
      await tester.pump();

      expect(launcher.calls, isEmpty);
    });

    testWidgets('rejects custom scheme without launching', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('assistant://chat/123');
      await tester.pump();

      expect(launcher.calls, isEmpty);
    });

    testWidgets('shows snackbar when launcher returns false', (tester) async {
      final launcher = _RecordingLauncher()..result = false;
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('https://example.com');
      await tester.pump();

      expect(find.textContaining('Could not open link'), findsOneWidget);
    });

    testWidgets('catches launcher exceptions and shows snackbar', (
      tester,
    ) async {
      final launcher = _RecordingLauncher()..throws = Exception('boom');
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('https://example.com');
      await tester.pump();

      expect(find.textContaining('Could not open link'), findsOneWidget);
    });

    testWidgets('rejects malformed url with snackbar', (tester) async {
      final launcher = _RecordingLauncher();
      late MarkdownLinkHandler handler;
      await _pump(tester, (context) {
        handler = MarkdownLinkHandler(
          context: context,
          launcher: launcher.call,
        );
      });

      await handler.onTap('not a url at all');
      await tester.pump();

      expect(launcher.calls, isEmpty);
      expect(find.textContaining('Cannot open link'), findsOneWidget);
    });
  });
}
