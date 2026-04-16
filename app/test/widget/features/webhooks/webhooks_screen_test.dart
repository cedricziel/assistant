import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/webhooks/webhooks_provider.dart';
import 'package:assistant_app/features/webhooks/webhooks_screen.dart';

void main() {
  group('WebhooksScreen', () {
    Widget buildUnderTest({required WebhooksState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const WebhooksScreen()),
          GoRoute(
            path: '/webhooks/:webhookId',
            builder: (_, _) => const Scaffold(body: Text('webhook detail')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          webhooksProvider.overrideWith(() => _FakeWebhooksNotifier(state)),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no webhooks', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const WebhooksState()));
      await tester.pump();

      expect(find.text('No webhooks configured'), findsOneWidget);
    });

    testWidgets('renders webhook rows for each webhook in state', (
      tester,
    ) async {
      final state = WebhooksState(
        webhooks: [
          _webhook('w1', 'My Hook', 'https://example.com/hook'),
          _webhook('w2', 'Other Hook', 'https://other.com/hook'),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('My Hook'), findsOneWidget);
      expect(find.text('Other Hook'), findsOneWidget);
    });

    testWidgets('shows webhook url as subtitle', (tester) async {
      final state = WebhooksState(
        webhooks: [_webhook('w1', 'Hook', 'https://example.com/callback')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('https://example.com/callback'), findsOneWidget);
    });

    testWidgets('shows Active badge for active webhook', (tester) async {
      final state = WebhooksState(
        webhooks: [
          _webhook('w1', 'Active Hook', 'https://a.com', active: true),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Active'), findsOneWidget);
    });

    testWidgets('shows Inactive badge for inactive webhook', (tester) async {
      final state = WebhooksState(
        webhooks: [
          _webhook('w1', 'Inactive Hook', 'https://a.com', active: false),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Inactive'), findsOneWidget);
    });

    testWidgets('shows verified icon for verified webhook', (tester) async {
      final state = WebhooksState(
        webhooks: [
          _webhook(
            'w1',
            'Verified Hook',
            'https://a.com',
            verifiedAt: DateTime(2024, 1, 1),
          ),
        ],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.byIcon(Icons.verified_outlined), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = WebhooksState(error: 'Server unreachable');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Server unreachable'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('tapping a webhook row navigates to webhook detail', (
      tester,
    ) async {
      final state = WebhooksState(
        webhooks: [_webhook('hook-abc', 'Nav Hook', 'https://nav.com')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      await tester.tap(find.text('Nav Hook'));
      await tester.pumpAndSettle();

      expect(find.text('webhook detail'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

WebhookResponse _webhook(
  String id,
  String name,
  String url, {
  bool active = true,
  DateTime? verifiedAt,
}) => WebhookResponse(
  (b) => b
    ..id = id
    ..name = name
    ..url = url
    ..active = active
    ..secret = 'secret'
    ..eventTypes = ListBuilder<String>(['message'])
    ..createdAt = DateTime(2024, 1, 1)
    ..updatedAt = DateTime(2024, 1, 1)
    ..verifiedAt = verifiedAt,
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeWebhooksNotifier extends WebhooksNotifier {
  _FakeWebhooksNotifier(this._state);
  final WebhooksState _state;

  @override
  Future<WebhooksState> build() async => _state;
}
