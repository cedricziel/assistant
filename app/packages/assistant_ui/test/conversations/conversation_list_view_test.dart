import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
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
    ValueChanged<ConversationActionRequest>? onRequestActions,
  }) => _host(
    ConversationListView(
      items: list ?? items,
      onSelect: onSelect ?? (_) {},
      onDelete: onDelete ?? (_) async => true,
      header: header,
      emptyState: emptyState,
      trailingBuilder: trailingBuilder,
      selectedId: selectedId,
      onRequestActions: onRequestActions,
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

  group('row actions', () {
    final overflow = find.byTooltip('Conversation actions');

    testWidgets('a touch long-press requests actions at the press position', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      await tester.pumpWidget(view(onRequestActions: requests.add));

      final target = tester.getCenter(find.text('Debug flaky CI job'));
      final gesture = await tester.startGesture(
        target,
        kind: PointerDeviceKind.touch,
      );
      await tester.pump(kLongPressTimeout + const Duration(milliseconds: 50));
      await gesture.up();
      await tester.pump();

      expect(requests, hasLength(1));
      expect(requests.single.item.id, '2');
      expect(requests.single.kind, PointerDeviceKind.touch);
      expect(requests.single.position, target);
      expect(requests.single.anchorRect, isNull);
    });

    testWidgets('a mouse secondary tap requests actions with kind mouse', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      await tester.pumpWidget(view(onRequestActions: requests.add));

      final target = tester.getCenter(find.text('Weekly notes'));
      await tester.tapAt(
        target,
        buttons: kSecondaryMouseButton,
        kind: PointerDeviceKind.mouse,
      );
      await tester.pump();

      expect(requests, hasLength(1));
      expect(requests.single.item.id, '3');
      expect(requests.single.kind, PointerDeviceKind.mouse);
      expect(requests.single.position, target);
      expect(requests.single.anchorRect, isNull);
    });

    testWidgets('a plain tap selects and does not request actions', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      final selected = <String>[];
      await tester.pumpWidget(
        view(onRequestActions: requests.add, onSelect: selected.add),
      );

      await tester.tap(find.text('Plan the launch'));
      await tester.pump();

      expect(selected, ['1']);
      expect(requests, isEmpty);
    });

    testWidgets('the overflow button is hidden until hover, focus or select', (
      tester,
    ) async {
      await tester.pumpWidget(view(onRequestActions: (_) {}));
      expect(overflow, findsNothing);

      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: Offset.zero);
      addTearDown(mouse.removePointer);
      await mouse.moveTo(tester.getCenter(find.text('Plan the launch')));
      await tester.pump();
      expect(overflow, findsOneWidget);

      await mouse.moveTo(const Offset(790, 590));
      await tester.pump();
      expect(overflow, findsNothing);
    });

    testWidgets('the overflow button shows on the selected row only', (
      tester,
    ) async {
      await tester.pumpWidget(view(selectedId: '2', onRequestActions: (_) {}));

      expect(overflow, findsOneWidget);
      expect(
        tester.getCenter(overflow).dy,
        closeTo(tester.getCenter(find.text('Debug flaky CI job')).dy, 24),
      );
    });

    testWidgets('the overflow button shows while its row has focus', (
      tester,
    ) async {
      await tester.pumpWidget(view(onRequestActions: (_) {}));

      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump();

      expect(overflow, findsOneWidget);
    });

    testWidgets('no overflow button without onRequestActions', (tester) async {
      await tester.pumpWidget(view(selectedId: '2'));

      expect(overflow, findsNothing);
    });

    testWidgets('the overflow button carries the semantics label', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await tester.pumpWidget(view(selectedId: '2', onRequestActions: (_) {}));

      expect(find.bySemanticsLabel('Conversation actions'), findsOneWidget);
      handle.dispose();
    });

    testWidgets(
      'a mouse click on the overflow button reports its anchor rect',
      (tester) async {
        final requests = <ConversationActionRequest>[];
        await tester.pumpWidget(
          view(selectedId: '2', onRequestActions: requests.add),
        );

        await tester.tap(overflow, kind: PointerDeviceKind.mouse);
        await tester.pump();

        expect(requests, hasLength(1));
        expect(requests.single.item.id, '2');
        expect(requests.single.kind, PointerDeviceKind.mouse);
        expect(requests.single.position, isNull);
        expect(
          requests.single.anchorRect,
          tester.getRect(find.byType(IconButton)),
        );
      },
    );

    testWidgets('a touch tap on the overflow button reports kind touch', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      await tester.pumpWidget(
        view(selectedId: '2', onRequestActions: requests.add),
      );

      await tester.tap(overflow, kind: PointerDeviceKind.touch);
      await tester.pump();

      expect(requests.single.kind, PointerDeviceKind.touch);
    });

    testWidgets('keyboard activation of the overflow button has a null kind', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      await tester.pumpWidget(
        view(selectedId: '2', onRequestActions: requests.add),
      );

      // Tab until focus lands inside the overflow button.
      final button = tester.element(overflow);
      var focused = false;
      for (var i = 0; i < 8 && !focused; i++) {
        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pump();
        FocusManager.instance.primaryFocus?.context?.visitAncestorElements((e) {
          focused = focused || e == button;
          return !focused;
        });
      }
      expect(focused, isTrue, reason: 'focus never reached the button');
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pump();

      expect(requests, hasLength(1));
      expect(requests.single.kind, isNull);
      expect(requests.single.position, isNull);
      expect(
        requests.single.anchorRect,
        tester.getRect(find.byType(IconButton)),
      );
    });

    testWidgets('a start-to-end swipe fires nothing and keeps the row', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      final deleted = <String>[];
      await tester.pumpWidget(
        view(
          onRequestActions: requests.add,
          onDelete: (id) async {
            deleted.add(id);
            return true;
          },
        ),
      );

      await tester.drag(find.text('Plan the launch'), const Offset(600, 0));
      await tester.pumpAndSettle();

      expect(requests, isEmpty);
      expect(deleted, isEmpty);
      expect(find.text('Plan the launch'), findsOneWidget);
    });

    testWidgets('an end-to-start swipe still goes through onDelete', (
      tester,
    ) async {
      final requests = <ConversationActionRequest>[];
      final deleted = <String>[];
      await tester.pumpWidget(
        view(
          onRequestActions: requests.add,
          onDelete: (id) async {
            deleted.add(id);
            return false;
          },
        ),
      );

      await tester.drag(find.text('Plan the launch'), const Offset(-600, 0));
      await tester.pumpAndSettle();

      expect(deleted, ['1']);
      expect(requests, isEmpty);
    });
  });
}
