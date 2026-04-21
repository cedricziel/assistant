import 'package:assistant_api/assistant_api.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/workflows/workflow_detail_screen.dart';
import 'package:assistant_app/features/workflows/workflows_provider.dart';

void main() {
  group('WorkflowDetailScreen', () {
    Widget buildUnderTest({
      required WorkflowDetailState detailState,
      _FakeWorkflowsNotifier? workflowsNotifier,
      _FakeWorkflowDetailNotifier? detailNotifier,
    }) {
      final fake = workflowsNotifier ?? _FakeWorkflowsNotifier();
      final router = GoRouter(
        routes: [
          GoRoute(
            path: '/',
            builder: (_, _) => const WorkflowDetailScreen(workflowId: 'wf-1'),
          ),
          GoRoute(
            path: '/workflows',
            builder: (_, _) => const Scaffold(body: Text('workflows list')),
          ),
          GoRoute(
            path: '/workflows/wf-1/edit',
            builder: (_, _) => const Scaffold(body: Text('edit screen')),
          ),
          GoRoute(
            path: '/workflows/wf-1/runs/:runId',
            builder: (_, state) =>
                Scaffold(body: Text('run ${state.pathParameters['runId']}')),
          ),
        ],
      );

      return ProviderScope(
        overrides: [
          workflowDetailProvider.overrideWith2(
            (arg) =>
                detailNotifier ?? _FakeWorkflowDetailNotifier(arg, detailState),
          ),
          workflowsProvider.overrideWith(() => fake),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    // -------------------------------------------------------------------------
    // Delete flow

    testWidgets('tapping delete shows confirmation dialog', (tester) async {
      await tester.pumpWidget(buildUnderTest(detailState: _activeState()));
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('Delete workflow?'), findsOneWidget);
    });

    testWidgets('canceling delete does not call deleteWorkflow', (
      tester,
    ) async {
      final notifier = _FakeWorkflowsNotifier();
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(notifier.deleteCalledWith, isNull);
      expect(find.byType(AlertDialog), findsNothing);
    });

    testWidgets('confirming delete calls deleteWorkflow with workflow id', (
      tester,
    ) async {
      final notifier = _FakeWorkflowsNotifier(deleteResult: 'simulated error');
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(notifier.deleteCalledWith, equals('wf-1'));
    });

    testWidgets('delete error shows snackbar', (tester) async {
      final notifier = _FakeWorkflowsNotifier(
        deleteResult: 'Something went wrong',
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.textContaining('Something went wrong'), findsOneWidget);
    });

    testWidgets('successful delete navigates to workflows list', (
      tester,
    ) async {
      final notifier = _FakeWorkflowsNotifier(deleteResult: null);
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.text('workflows list'), findsOneWidget);
    });

    // -------------------------------------------------------------------------
    // Active / inactive toggle

    testWidgets('tapping chip on active workflow calls deactivateWorkflow', (
      tester,
    ) async {
      final notifier = _FakeWorkflowsNotifier(
        deactivateResult: 'simulated error',
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      // Tap the Active/Inactive status chip.
      await tester.tap(find.text('Active'));
      await tester.pumpAndSettle();

      expect(notifier.deactivateCalledWith, equals('wf-1'));
      expect(notifier.activateCalledWith, isNull);
    });

    testWidgets('tapping chip on inactive workflow calls activateWorkflow', (
      tester,
    ) async {
      final notifier = _FakeWorkflowsNotifier(
        activateResult: 'simulated error',
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _inactiveState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.text('Inactive'));
      await tester.pumpAndSettle();

      expect(notifier.activateCalledWith, equals('wf-1'));
      expect(notifier.deactivateCalledWith, isNull);
    });

    testWidgets('toggle error shows snackbar', (tester) async {
      final notifier = _FakeWorkflowsNotifier(activateResult: 'Toggle failed');
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _inactiveState(),
          workflowsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.text('Inactive'));
      await tester.pumpAndSettle();

      expect(find.textContaining('Toggle failed'), findsOneWidget);
    });

    // -------------------------------------------------------------------------
    // Test run

    testWidgets('test run success shows snackbar with message', (tester) async {
      final preview = WorkflowRunPreview(
        (b) => b
          ..runId = 'run-42'
          ..workflowId = 'wf-1'
          ..status = 'running'
          ..message = 'Test run started'
          ..maxSteps = 50
          ..maxVisitsPerNode = 3
          ..startedAt = DateTime(2024, 1, 1),
      );
      final detailNotifier = _FakeWorkflowDetailNotifier(
        'wf-1',
        _activeState(),
        testRunResult: (preview, null),
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          detailNotifier: detailNotifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.play_arrow));
      await tester.pumpAndSettle();

      expect(
        find.textContaining('Test run started'),
        findsOneWidget,
        reason: 'success snackbar should show preview message',
      );
    });

    testWidgets('test run failure shows error snackbar', (tester) async {
      final detailNotifier = _FakeWorkflowDetailNotifier(
        'wf-1',
        _activeState(),
        testRunResult: (null, 'Connection refused'),
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _activeState(),
          detailNotifier: detailNotifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.play_arrow));
      await tester.pumpAndSettle();

      expect(
        find.textContaining('Connection refused'),
        findsOneWidget,
        reason: 'error snackbar should show failure message',
      );
    });

    // -------------------------------------------------------------------------
    // Webhook secrets

    testWidgets('webhook trigger workflow shows webhook secrets card', (
      tester,
    ) async {
      final secrets = WorkflowWebhookSecrets(
        (b) => b
          ..webhookUrl = 'https://example.com/hook/wf-1'
          ..webhookToken = 'secret-token-abc',
      );
      final detailNotifier = _FakeWorkflowDetailNotifier(
        'wf-1',
        _webhookState(),
        webhookSecrets: secrets,
      );
      await tester.pumpWidget(
        buildUnderTest(
          detailState: _webhookState(),
          detailNotifier: detailNotifier,
        ),
      );
      await tester.pump();
      // Allow the lazy fetch to complete.
      await tester.pump();

      expect(
        find.text('Webhook'),
        findsOneWidget,
        reason: 'webhook secrets card header should appear',
      );
      expect(
        find.text('https://example.com/hook/wf-1'),
        findsOneWidget,
        reason: 'webhook URL should be displayed',
      );
      // Token should be obscured by default.
      expect(
        find.text('secret-token-abc'),
        findsNothing,
        reason: 'token should be hidden by default',
      );
    });

    testWidgets('non-webhook workflow does not show webhook card', (
      tester,
    ) async {
      await tester.pumpWidget(buildUnderTest(detailState: _activeState()));
      await tester.pump();

      expect(
        find.text('Webhook'),
        findsNothing,
        reason: 'webhook card should not appear for non-webhook workflows',
      );
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

WorkflowDetailState _activeState() => WorkflowDetailState(
  detail: WorkflowDetail(
    (b) => b
      ..id = 'wf-1'
      ..name = 'My Workflow'
      ..description = 'A workflow'
      ..active = true
      ..createdAt = DateTime(2024, 1, 1)
      ..updatedAt = DateTime(2024, 1, 1),
  ),
  runs: const [],
);

WorkflowDetailState _inactiveState() => WorkflowDetailState(
  detail: WorkflowDetail(
    (b) => b
      ..id = 'wf-1'
      ..name = 'My Workflow'
      ..description = 'A workflow'
      ..active = false
      ..createdAt = DateTime(2024, 1, 1)
      ..updatedAt = DateTime(2024, 1, 1),
  ),
  runs: const [],
);

WorkflowDetailState _webhookState() => WorkflowDetailState(
  detail: WorkflowDetail(
    (b) => b
      ..id = 'wf-1'
      ..name = 'Webhook Workflow'
      ..description = 'A webhook-triggered workflow'
      ..active = true
      ..createdAt = DateTime(2024, 1, 1)
      ..updatedAt = DateTime(2024, 1, 1)
      ..graph = JsonObject({
        'nodes': [
          {
            'id': 'trigger-1',
            'kind': 'trigger',
            'config': {'type': 'webhook'},
          },
        ],
        'edges': [],
      }),
  ),
  runs: const [],
);

// ---------------------------------------------------------------------------
// Fake notifiers

class _FakeWorkflowDetailNotifier extends WorkflowDetailNotifier {
  _FakeWorkflowDetailNotifier(
    super.workflowId,
    this._state, {
    this.testRunResult,
    this.webhookSecrets,
  });

  final WorkflowDetailState _state;
  final (WorkflowRunPreview?, String?)? testRunResult;
  final WorkflowWebhookSecrets? webhookSecrets;

  @override
  Future<WorkflowDetailState> build() async => _state;

  @override
  Future<void> refresh() async {}

  @override
  Future<(WorkflowRunPreview?, String?)> testRun() async =>
      testRunResult ?? (null, 'Not configured');

  @override
  Future<WorkflowWebhookSecrets?> fetchWebhookSecrets() async => webhookSecrets;
}

class _FakeWorkflowsNotifier extends WorkflowsNotifier {
  _FakeWorkflowsNotifier({
    this.deleteResult,
    this.activateResult,
    this.deactivateResult,
  });

  final String? deleteResult;
  final String? activateResult;
  final String? deactivateResult;

  String? deleteCalledWith;
  String? activateCalledWith;
  String? deactivateCalledWith;

  @override
  Future<List<WorkflowSummary>> build() async => [];

  @override
  Future<String?> deleteWorkflow(String id) async {
    deleteCalledWith = id;
    return deleteResult;
  }

  @override
  Future<String?> activateWorkflow(String id) async {
    activateCalledWith = id;
    return activateResult;
  }

  @override
  Future<String?> deactivateWorkflow(String id) async {
    deactivateCalledWith = id;
    return deactivateResult;
  }
}
