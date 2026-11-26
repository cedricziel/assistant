import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

ConversationListItem _item(
  String id,
  String title,
  Duration ago, {
  DateTime? pinnedAt,
}) => ConversationListItem(
  id: id,
  title: title,
  updatedAt: _now.subtract(ago),
  pinnedAt: pinnedAt,
);

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  final items = [
    _item('1', 'Plan the launch', const Duration(seconds: 20)),
    _item('2', 'Debug flaky CI job', const Duration(minutes: 5, seconds: 30)),
    _item('3', 'Weekly notes', const Duration(hours: 2, minutes: 30)),
    _item('4', 'Summarise meeting', const Duration(days: 3, hours: 12)),
  ];

  Widget view({
    List<ConversationListItem>? list,
    ValueChanged<String>? onSelect,
    Future<bool> Function(String id)? onDelete,
    Widget? header,
    Widget? emptyState,
    ConversationTrailingBuilder? trailingBuilder,
    String? selectedId,
  }) => _host(
    ConversationListView(
      items: list ?? items,
      onSelect: onSelect ?? (_) {},
      onDelete: onDelete ?? (_) async => true,
      header: header,
      emptyState: emptyState,
      trailingBuilder: trailingBuilder,
      selectedId: selectedId,
      now: _now,
    ),
  );

  testWidgets('renders title and relative date against the injected now', (
    tester,
  ) async {
    await tester.pumpWidget(view());

    expect(find.text('Plan the launch'), findsOneWidget);
    expect(find.text('Just now'), findsOneWidget);
    expect(find.text('5m ago'), findsOneWidget);
    expect(find.text('2h ago'), findsOneWidget);
    expect(find.text('3d ago'), findsOneWidget);
  });

  testWidgets('a Pinned header precedes the pinned rows', (tester) async {
    await tester.pumpWidget(
      view(
        list: [
          _item(
            'p',
            'Pinned one',
            const Duration(days: 9),
            pinnedAt: _now.subtract(const Duration(days: 1)),
          ),
          _item('1', 'Plan the launch', const Duration(seconds: 20)),
          _item('2', 'Debug flaky CI job', const Duration(minutes: 5)),
        ],
      ),
    );

    expect(find.text('Pinned'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('Pinned')).dy,
      lessThan(tester.getTopLeft(find.text('Pinned one')).dy),
    );
    expect(
      tester.getTopLeft(find.text('Pinned one')).dy,
      lessThan(tester.getTopLeft(find.text('Plan the launch')).dy),
    );
  });

  testWidgets('no Pinned header when nothing is pinned', (tester) async {
    await tester.pumpWidget(view());

    expect(find.text('Pinned'), findsNothing);
  });

  testWidgets('only the row whose id equals selectedId is highlighted', (
    tester,
  ) async {
    await tester.pumpWidget(view(selectedId: '2'));

    final tiles = tester.widgetList<ListTile>(find.byType(ListTile)).toList();
    expect(tiles.map((t) => t.selected), [false, true, false, false]);
  });

  testWidgets('no row is highlighted without a selectedId', (tester) async {
    await tester.pumpWidget(view());

    expect(
      tester.widgetList<ListTile>(find.byType(ListTile)).any((t) => t.selected),
      isFalse,
    );
  });

  testWidgets('a tap fires onSelect with the id and navigates nowhere', (
    tester,
  ) async {
    final selected = <String>[];
    await tester.pumpWidget(view(onSelect: selected.add));

    await tester.tap(find.text('Debug flaky CI job'));
    await tester.pump();

    expect(selected, ['2']);
  });

  testWidgets('a swipe resolving true dismisses the row', (tester) async {
    final asked = <String>[];
    late StateSetter setList;
    var current = items;
    await tester.pumpWidget(
      _host(
        StatefulBuilder(
          builder: (context, setState) {
            setList = setState;
            return ConversationListView(
              items: current,
              onSelect: (_) {},
              onDelete: (id) async {
                asked.add(id);
                setList(
                  () => current = current.where((i) => i.id != id).toList(),
                );
                return true;
              },
              now: _now,
            );
          },
        ),
      ),
    );

    await tester.drag(find.text('Plan the launch'), const Offset(-600, 0));
    await tester.pumpAndSettle();

    expect(asked, ['1']);
    expect(find.text('Plan the launch'), findsNothing);
    expect(find.text('Debug flaky CI job'), findsOneWidget);
  });

  testWidgets('a swipe resolving false keeps the row and no dismissed row', (
    tester,
  ) async {
    final asked = <String>[];
    await tester.pumpWidget(
      view(
        onDelete: (id) async {
          asked.add(id);
          return false;
        },
      ),
    );

    await tester.drag(find.text('Plan the launch'), const Offset(-600, 0));
    await tester.pumpAndSettle();

    expect(asked, ['1']);
    expect(find.text('Plan the launch'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('an empty list renders the header and no rows', (tester) async {
    await tester.pumpWidget(
      view(list: const [], header: const Text('Conversations')),
    );

    expect(find.text('Conversations'), findsOneWidget);
    expect(find.byType(ListTile), findsNothing);
    expect(find.byType(Dismissible), findsNothing);
  });

  testWidgets('an empty list renders the emptyState slot', (tester) async {
    await tester.pumpWidget(
      view(list: const [], emptyState: const Text('Nothing here')),
    );

    expect(find.text('Nothing here'), findsOneWidget);
  });

  testWidgets('the header renders above the rows', (tester) async {
    await tester.pumpWidget(view(header: const Text('Conversations')));

    expect(
      tester.getTopLeft(find.text('Conversations')).dy,
      lessThan(tester.getTopLeft(find.text('Plan the launch')).dy),
    );
  });

  testWidgets('trailingBuilder renders once per row', (tester) async {
    await tester.pumpWidget(
      view(
        trailingBuilder: (context, item) =>
            Text('more-${item.id}', key: Key('trailing_${item.id}')),
      ),
    );

    expect(find.byKey(const Key('trailing_1')), findsOneWidget);
    expect(find.byKey(const Key('trailing_4')), findsOneWidget);
  });
}
