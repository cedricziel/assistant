import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/logs/logs_provider.dart';
import 'package:assistant_app/features/logs/logs_screen.dart';

void main() {
  group('LogsScreen', () {
    Widget buildUnderTest({required LogsState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const LogsScreen()),
          GoRoute(
            path: '/chat',
            builder: (_, _) => const Scaffold(body: Text('chat')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [logsProvider.overrideWith(() => _FakeLogsNotifier(state))],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no logs', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const LogsState()));
      await tester.pump();

      expect(find.text('No log entries'), findsOneWidget);
    });

    testWidgets('renders log rows with message and severity', (tester) async {
      final state = LogsState(
        logs: [
          _logEntry(id: '1', message: 'Server started', severity: 'INFO'),
          _logEntry(id: '2', message: 'Connection failed', severity: 'ERROR'),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Server started'), findsOneWidget);
      expect(find.text('Connection failed'), findsOneWidget);
      // Severity chips are truncated to 4 chars
      expect(find.text('INFO'), findsOneWidget);
      expect(find.text('ERRO'), findsOneWidget);
    });

    testWidgets('renders WARN severity chip', (tester) async {
      final state = LogsState(
        logs: [_logEntry(id: '1', message: 'slow', severity: 'WARN')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('WARN'), findsOneWidget);
    });

    testWidgets('renders DEBUG severity chip', (tester) async {
      final state = LogsState(
        logs: [_logEntry(id: '1', message: 'trace', severity: 'DEBUG')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('DEBU'), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = LogsState(error: 'Fetch failed');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Fetch failed'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('shows no-match message when search has no results', (
      tester,
    ) async {
      final state = LogsState(logs: const [], searchQuery: 'foobar');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.textContaining('No logs matching'), findsOneWidget);
      expect(find.textContaining('foobar'), findsOneWidget);
    });

    testWidgets('search field is present with correct key', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const LogsState()));
      await tester.pump();

      expect(find.byKey(const Key('log_search_field')), findsOneWidget);
    });

    testWidgets('shows target module below message when non-empty', (
      tester,
    ) async {
      final state = LogsState(
        logs: [
          _logEntry(
            id: '1',
            message: 'booting',
            severity: 'INFO',
            target: 'assistant::runtime',
          ),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('assistant::runtime'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

LogEntryResponse _logEntry({
  required String id,
  required String message,
  required String severity,
  String target = '',
}) => LogEntryResponse(
  (b) => b
    ..id = id
    ..message = message
    ..severity = severity
    ..target = target
    ..timestamp = DateTime(2024, 1, 1, 12, 0, 0),
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeLogsNotifier extends LogsNotifier {
  _FakeLogsNotifier(this._state);
  final LogsState _state;

  @override
  Future<LogsState> build() async => _state;
}
