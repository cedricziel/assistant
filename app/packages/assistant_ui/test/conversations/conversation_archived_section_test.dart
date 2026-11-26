import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

ConversationListItem _item(String id, String title, {bool archived = false}) =>
    ConversationListItem(
      id: id,
      title: title,
      updatedAt: _now.subtract(const Duration(days: 2)),
      archivedAt: archived ? _now.subtract(const Duration(days: 1)) : null,
    );

void main() {
  final live = [_item('1', 'Plan the launch'), _item('2', 'Weekly notes')];
  final archived = [
    _item('a1', 'Old trip planning', archived: true),
    _item('a2', 'Resolved incident', archived: true),
  ];

  Widget host({
    List<ConversationListItem>? list,
    bool expanded = false,
    ValueChanged<bool>? onExpandedChanged,
    List<ConversationListItem>? archivedItems,
    bool loading = false,
    String? error,
    VoidCallback? onRetry,
    ValueChanged<String>? onSelect,
    Future<bool> Function(String id)? onDelete,
    ValueChanged<ConversationActionRequest>? onRequestActions,
    Widget? emptyState,
  }) => MaterialApp(
    home: Scaffold(
      body: ConversationListView(
        items: list ?? live,
        onSelect: (_) {},
        onDelete: (_) async => true,
        emptyState: emptyState,
        now: _now,
        archivedSection: ConversationArchivedSection(
          expanded: expanded,
          onExpandedChanged: onExpandedChanged ?? (_) {},
          items: archivedItems ?? const [],
          loading: loading,
          error: error,
          onRetry: onRetry,
          onSelect: onSelect ?? (_) {},
          onDelete: onDelete ?? (_) async => true,
          onRequestActions: onRequestActions,
          now: _now,
        ),
      ),
    ),
  );

  testWidgets('ConversationListItem carries archivedAt', (tester) async {
    expect(_item('x', 'T').archivedAt, isNull);
    expect(_item('x', 'T', archived: true).archivedAt, isNotNull);
  });

  testWidgets('renders a collapsed Archived header below the rows', (
    tester,
  ) async {
    await tester.pumpWidget(host(archivedItems: archived));

    expect(find.text('Archived'), findsOneWidget);
    expect(find.text('Old trip planning'), findsNothing);
    final rowY = tester.getTopLeft(find.text('Weekly notes')).dy;
    final headerY = tester.getTopLeft(find.text('Archived')).dy;
    expect(headerY, greaterThan(rowY));
  });

  testWidgets('tapping the header requests expansion', (tester) async {
    final calls = <bool>[];
    await tester.pumpWidget(host(onExpandedChanged: calls.add));

    await tester.tap(find.text('Archived'));

    expect(calls, [true]);
  });

  testWidgets('tapping an expanded header requests collapse', (tester) async {
    final calls = <bool>[];
    await tester.pumpWidget(
      host(
        expanded: true,
        archivedItems: archived,
        onExpandedChanged: calls.add,
      ),
    );

    await tester.tap(find.text('Archived'));

    expect(calls, [false]);
  });

  testWidgets('the header reports its expanded state to semantics', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(host());
    expect(
      tester.getSemantics(find.text('Archived')),
      isSemantics(isExpanded: false),
    );

    await tester.pumpWidget(host(expanded: true));
    expect(
      tester.getSemantics(find.text('Archived')),
      isSemantics(isExpanded: true),
    );
    handle.dispose();
  });

  testWidgets('expanded, renders the archived rows', (tester) async {
    await tester.pumpWidget(host(expanded: true, archivedItems: archived));

    expect(find.text('Old trip planning'), findsOneWidget);
    expect(find.text('Resolved incident'), findsOneWidget);
    expect(find.text('2d ago'), findsNWidgets(4));
  });

  testWidgets('an archived row selects, deletes and requests actions', (
    tester,
  ) async {
    final selected = <String>[];
    final deleted = <String>[];
    final requests = <ConversationActionRequest>[];
    await tester.pumpWidget(
      host(
        expanded: true,
        archivedItems: archived,
        onSelect: selected.add,
        onDelete: (id) async {
          deleted.add(id);
          return true;
        },
        onRequestActions: requests.add,
      ),
    );

    await tester.tap(find.text('Old trip planning'));
    expect(selected, ['a1']);

    await tester.longPress(find.text('Old trip planning'));
    expect(requests.single.item.id, 'a1');
    expect(requests.single.item.archivedAt, isNotNull);

    await tester.drag(find.text('Resolved incident'), const Offset(-600, 0));
    await tester.pumpAndSettle();
    expect(deleted, ['a2']);
    expect(find.text('Resolved incident'), findsNothing);
  });

  testWidgets('shows a spinner while loading and nothing is loaded', (
    tester,
  ) async {
    await tester.pumpWidget(host(expanded: true, loading: true));

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });

  testWidgets('keeps the rows visible while refetching', (tester) async {
    await tester.pumpWidget(
      host(expanded: true, loading: true, archivedItems: archived),
    );

    expect(find.text('Old trip planning'), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
  });

  testWidgets('shows an error with a working Retry', (tester) async {
    var retries = 0;
    await tester.pumpWidget(
      host(expanded: true, error: 'Failed to load', onRetry: () => retries++),
    );

    expect(find.text('Failed to load'), findsOneWidget);
    await tester.tap(find.text('Retry'));
    expect(retries, 1);
  });

  testWidgets('shows an empty note when nothing is archived', (tester) async {
    await tester.pumpWidget(host(expanded: true));

    expect(find.text('No archived conversations'), findsOneWidget);
  });

  testWidgets('the slot stays visible when there are no live rows', (
    tester,
  ) async {
    await tester.pumpWidget(
      host(
        list: const [],
        emptyState: const Center(child: Text('Nothing yet')),
        expanded: true,
        archivedItems: archived,
      ),
    );

    expect(find.text('Nothing yet'), findsOneWidget);
    expect(find.text('Archived'), findsOneWidget);
    expect(find.text('Old trip planning'), findsOneWidget);
  });
}
