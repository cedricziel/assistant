import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/workflows/workflow_editor_screen.dart';
import 'package:assistant_app/features/workflows/workflows_provider.dart';

void main() {
  // ---------------------------------------------------------------------------
  // Unit tests

  group('buildMobileEdges', () {
    EditorNode node(String id, String kind) => EditorNode(
      id: id,
      kind: kind,
      config: {'type': kind == 'trigger' ? 'manual' : 'http_request'},
      position: Offset.zero,
    );

    test('empty list produces no edges', () {
      expect(buildMobileEdges([]), isEmpty);
    });

    test('single node produces no edges', () {
      expect(buildMobileEdges([node('a', 'trigger')]), isEmpty);
    });

    test('[trigger, action1, action2] produces two edges in correct order', () {
      final trigger = node('t', 'trigger');
      final action1 = node('a1', 'action');
      final action2 = node('a2', 'action');
      final edges = buildMobileEdges([trigger, action1, action2]);

      expect(edges.length, 2, reason: 'expected 2 edges for 3-node chain');
      expect(edges[0].from, 't', reason: 'first edge originates from trigger');
      expect(edges[0].to, 'a1', reason: 'first edge points to action1');
      expect(edges[0].on, 'trigger', reason: 'trigger outcome label');
      expect(edges[1].from, 'a1', reason: 'second edge from action1');
      expect(edges[1].to, 'a2', reason: 'second edge to action2');
      expect(edges[1].on, 'success', reason: 'action outcome label');
    });

    test('condition node uses "true" as primary outcome', () {
      final trigger = node('t', 'trigger');
      final cond = node('c', 'condition');
      final action = node('a', 'action');
      final edges = buildMobileEdges([trigger, cond, action]);

      expect(edges[1].from, 'c');
      expect(edges[1].to, 'a');
      expect(edges[1].on, 'true', reason: 'condition default branch is true');
    });
  });

  group('isComplexDag', () {
    EditorNode node(String id) =>
        EditorNode(id: id, kind: 'action', config: {}, position: Offset.zero);

    test('returns false for empty graph', () {
      expect(isComplexDag([], []), isFalse);
    });

    test('returns false for linear chain', () {
      final nodes = [node('a'), node('b'), node('c')];
      final edges = [
        EditorEdge(from: 'a', to: 'b', on: 'success'),
        EditorEdge(from: 'b', to: 'c', on: 'success'),
      ];
      expect(isComplexDag(nodes, edges), isFalse);
    });

    test('returns true for fan-in graph (two edges into one node)', () {
      final nodes = [node('a'), node('b'), node('c')];
      final edges = [
        EditorEdge(from: 'a', to: 'c', on: 'success'),
        EditorEdge(from: 'b', to: 'c', on: 'success'),
      ];
      expect(isComplexDag(nodes, edges), isTrue);
    });

    test('returns true when a non-first node is disconnected', () {
      final nodes = [node('a'), node('b'), node('c')];
      final edges = [
        EditorEdge(from: 'a', to: 'b', on: 'success'),
        // 'c' has no edge → disconnected
      ];
      expect(isComplexDag(nodes, edges), isTrue);
    });
  });

  group('EditorNode.toJson', () {
    test('includes position in output', () {
      final node = EditorNode(
        id: 'n1',
        kind: 'action',
        config: {'type': 'http_request', 'url': 'https://example.com'},
        position: const Offset(100.5, 250.0),
      );
      final json = node.toJson();

      expect(json['id'], 'n1');
      expect(json['kind'], 'action');
      expect(json['config'], isA<Map>());
      expect(json['position'], isA<Map>());
      expect(json['position']['x'], 100.5);
      expect(json['position']['y'], 250.0);
    });
  });

  // ---------------------------------------------------------------------------
  // Widget tests

  group('WorkflowEditorScreen layout', () {
    Widget buildScreen() {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const WorkflowEditorScreen()),
          GoRoute(
            path: '/workflows',
            builder: (_, _) => const Scaffold(body: Text('workflows')),
          ),
        ],
      );

      return ProviderScope(
        overrides: [
          workflowsProvider.overrideWith(() => _FakeWorkflowsNotifier()),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets(
      'narrow viewport (width=400) shows mobile card list and hides canvas',
      (tester) async {
        tester.view.physicalSize = const Size(400, 844);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);

        await tester.pumpWidget(buildScreen());
        await tester.pump();

        expect(
          find.byType(ReorderableListView),
          findsOneWidget,
          reason: 'mobile card list should be shown',
        );
        expect(
          find.byType(InteractiveViewer),
          findsNothing,
          reason: 'canvas should be hidden on narrow viewport',
        );
      },
    );

    testWidgets(
      'wide viewport (width=800) shows canvas and hides mobile list',
      (tester) async {
        tester.view.physicalSize = const Size(800, 600);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);

        await tester.pumpWidget(buildScreen());
        await tester.pump();

        expect(
          find.byType(InteractiveViewer),
          findsOneWidget,
          reason: 'canvas should be shown on wide viewport',
        );
        expect(
          find.byType(ReorderableListView),
          findsNothing,
          reason: 'mobile list should be hidden on wide viewport',
        );
      },
    );

    testWidgets('add node in mobile view appends card to list', (tester) async {
      tester.view.physicalSize = const Size(400, 844);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(buildScreen());
      await tester.pump();

      // Pre-populated trigger card
      expect(find.text('Manual Trigger'), findsOneWidget);

      // Open palette
      await tester.tap(find.text('Add step'));
      await tester.pumpAndSettle();

      // Select HTTP Request action
      await tester.tap(find.text('HTTP Request'));
      await tester.pumpAndSettle();

      expect(
        find.text('Manual Trigger'),
        findsOneWidget,
        reason: 'trigger card still present',
      );
      expect(
        find.text('HTTP Request'),
        findsOneWidget,
        reason: 'new action card appended',
      );
    });

    testWidgets(
      'edit button on mobile card opens config sheet with correct title',
      (tester) async {
        // 500px: still mobile layout (<600) but wide enough for the sheet header.
        tester.view.physicalSize = const Size(500, 844);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);

        await tester.pumpWidget(buildScreen());
        await tester.pump();

        // Tap the edit pencil on the pre-populated trigger card.
        await tester.tap(find.byIcon(Icons.edit_outlined).first);
        await tester.pumpAndSettle();

        // The bottom-sheet header should show the node's label.
        expect(
          find.textContaining('Configure:'),
          findsOneWidget,
          reason: 'config sheet should open with a Configure: header',
        );
      },
    );

    testWidgets(
      'editing http_request node URL via config sheet keeps card in list',
      (tester) async {
        // 500px: still mobile layout (<600) but wide enough for the config sheet.
        // Use 1000px height to ensure both cards are on-screen with default-density buttons.
        tester.view.physicalSize = const Size(500, 1000);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);

        await tester.pumpWidget(buildScreen());
        await tester.pump();

        // Add an HTTP Request node.
        await tester.tap(find.text('Add step'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('HTTP Request'));
        await tester.pumpAndSettle();

        // Tap edit on the new HTTP Request card (last edit icon in the list).
        await tester.tap(find.byIcon(Icons.edit_outlined).last);
        await tester.pumpAndSettle();

        // Fill in a URL in the first text field of the config sheet.
        await tester.enterText(
          find.byType(TextField).first,
          'https://example.com',
        );

        // Confirm via Apply.
        await tester.tap(find.text('Apply'));
        await tester.pumpAndSettle();

        // Both cards remain present after editing.
        expect(find.text('Manual Trigger'), findsOneWidget);
        expect(find.text('HTTP Request'), findsOneWidget);
      },
    );

    testWidgets('desktop FAB opens node palette listing trigger options', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(800, 600);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(buildScreen());
      await tester.pump();

      expect(find.byType(InteractiveViewer), findsOneWidget);

      // Tap the "Add node" FloatingActionButton to open the node palette.
      await tester.tap(find.byTooltip('Add node'));
      await tester.pumpAndSettle();

      // The section header is uppercased by _PaletteSection; items are title-cased.
      expect(
        find.text('TRIGGERS'),
        findsOneWidget,
        reason: 'palette Triggers section header should appear (uppercased)',
      );
      expect(
        find.text('Manual'),
        findsOneWidget,
        reason: 'Manual trigger item should be listed',
      );
    });

    testWidgets(
      'delete node in mobile view shows confirmation then removes card',
      (tester) async {
        tester.view.physicalSize = const Size(400, 844);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);

        await tester.pumpWidget(buildScreen());
        await tester.pump();

        expect(find.text('Manual Trigger'), findsOneWidget);

        // Tap delete on the trigger card — opens confirmation dialog.
        await tester.tap(find.byIcon(Icons.delete_outline).first);
        await tester.pumpAndSettle();

        expect(
          find.text('Delete node?'),
          findsOneWidget,
          reason: 'confirmation dialog should appear',
        );

        // Confirm deletion.
        await tester.tap(find.text('Delete'));
        await tester.pumpAndSettle();

        expect(
          find.text('Manual Trigger'),
          findsNothing,
          reason: 'card should be removed after confirming delete',
        );
      },
    );

    testWidgets('canceling delete keeps the node in list', (tester) async {
      tester.view.physicalSize = const Size(400, 844);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(buildScreen());
      await tester.pump();

      expect(find.text('Manual Trigger'), findsOneWidget);

      // Tap delete on the trigger card.
      await tester.tap(find.byIcon(Icons.delete_outline).first);
      await tester.pumpAndSettle();

      // Cancel the confirmation dialog.
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(
        find.text('Manual Trigger'),
        findsOneWidget,
        reason: 'card should remain after canceling delete',
      );
    });
  });
}

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeWorkflowsNotifier extends WorkflowsNotifier {
  @override
  Future<List<WorkflowSummary>> build() async => [];

  @override
  Future<String?> createWorkflow({
    required String name,
    String? description,
    required Map<String, dynamic> graph,
    bool active = false,
  }) async => null;

  @override
  Future<String?> updateWorkflow({
    required String id,
    required String name,
    String? description,
    required Map<String, dynamic> graph,
    bool? active,
  }) async => null;
}
