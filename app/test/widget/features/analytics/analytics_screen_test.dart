import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/analytics/analytics_provider.dart';
import 'package:assistant_app/features/analytics/analytics_screen.dart';

void main() {
  group('AnalyticsScreen', () {
    Widget buildUnderTest({required AnalyticsState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const AnalyticsScreen()),
        ],
      );
      return ProviderScope(
        overrides: [
          analyticsProvider.overrideWith(() => _FakeAnalyticsNotifier(state)),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows no-data message when summary is null', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const AnalyticsState()));
      await tester.pump();

      expect(find.text('No analytics data'), findsOneWidget);
    });

    testWidgets('shows time window chips', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const AnalyticsState()));
      await tester.pump();

      expect(find.text('1h'), findsOneWidget);
      expect(find.text('24h'), findsOneWidget);
      expect(find.text('7d'), findsOneWidget);
    });

    testWidgets('renders summary cards when data is present', (tester) async {
      final state = AnalyticsState(summary: _summary(requests: 42));
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('42'), findsOneWidget);
      expect(find.text('Requests'), findsOneWidget);
    });

    testWidgets('formats large token counts with k suffix', (tester) async {
      final state = AnalyticsState(summary: _summary(tokensIn: 15000));
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('15.0k'), findsOneWidget);
    });

    testWidgets('shows model breakdown section when models present', (
      tester,
    ) async {
      final model = ModelUsageResponse(
        (b) => b
          ..model = 'claude-3'
          ..requestCount = 5
          ..inputTokens = 100
          ..outputTokens = 200
          ..avgDurationS = 1.0,
      );
      final state = AnalyticsState(summary: _summary(models: [model]));
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Model Breakdown', skipOffstage: false), findsOneWidget);
      expect(find.text('claude-3', skipOffstage: false), findsOneWidget);
    });

    testWidgets('shows tool usage section when tools present', (tester) async {
      final tool = ToolUsageResponse(
        (b) => b
          ..toolName = 'file-read'
          ..invocations = 7
          ..avgDurationS = 0.2,
      );
      final state = AnalyticsState(summary: _summary(tools: [tool]));
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Tool Usage', skipOffstage: false), findsOneWidget);
      expect(find.text('file-read', skipOffstage: false), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = AnalyticsState(error: 'Analytics unavailable');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Analytics unavailable'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

AnalyticsSummaryResponse _summary({
  int requests = 0,
  int tokensIn = 0,
  int tokensOut = 0,
  int toolInvocations = 0,
  int errors = 0,
  double avgDuration = 0.0,
  List<ModelUsageResponse> models = const [],
  List<ToolUsageResponse> tools = const [],
}) => AnalyticsSummaryResponse(
  (b) => b
    ..totalRequests = requests
    ..totalTokensIn = tokensIn
    ..totalTokensOut = tokensOut
    ..totalToolInvocations = toolInvocations
    ..errorCount = errors
    ..avgDurationS = avgDuration
    ..windowHours = 24
    ..uniqueModels = ListBuilder<String>()
    ..models = ListBuilder<ModelUsageResponse>(models)
    ..tools = ListBuilder<ToolUsageResponse>(tools)
    ..requestSeries = ListBuilder<TimeSeriesResponse>()
    ..tokenSeries = ListBuilder<TimeSeriesResponse>(),
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeAnalyticsNotifier extends AnalyticsNotifier {
  _FakeAnalyticsNotifier(this._state);
  final AnalyticsState _state;

  @override
  Future<AnalyticsState> build() async => _state;
}
