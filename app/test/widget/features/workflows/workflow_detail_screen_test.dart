import 'package:assistant_api/assistant_api.dart';
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
        ],
      );

      return ProviderScope(
        overrides: [
          workflowDetailProvider.overrideWith2(
            (arg) => _FakeWorkflowDetailNotifier(arg, detailState),
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

// ---------------------------------------------------------------------------
// Fake notifiers

class _FakeWorkflowDetailNotifier extends WorkflowDetailNotifier {
  _FakeWorkflowDetailNotifier(super.workflowId, this._state);

  final WorkflowDetailState _state;

  @override
  Future<WorkflowDetailState> build() async => _state;

  @override
  Future<void> refresh() async {}
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
