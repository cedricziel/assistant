import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';

Widget _box(String key, {double height = 20}) =>
    SizedBox(key: Key(key), height: height);

Future<void> _pump(WidgetTester tester, ChatThreadLayout layout) =>
    tester.pumpWidget(MaterialApp(home: Scaffold(body: layout)));

void main() {
  group('ChatThreadLayout', () {
    testWidgets('stacks banners, thread, aboveComposer and composer', (
      tester,
    ) async {
      await _pump(
        tester,
        ChatThreadLayout(
          banners: [_box('error'), _box('history')],
          thread: _box('thread', height: 10),
          aboveComposer: [_box('reconnect'), _box('progress')],
          composer: _box('composer'),
        ),
      );

      final order = [
        'error',
        'history',
        'thread',
        'reconnect',
        'progress',
        'composer',
      ];
      final tops = [
        for (final key in order) tester.getTopLeft(find.byKey(Key(key))).dy,
      ];
      expect(tops, [...tops]..sort(), reason: 'top to bottom: $order');
      expect(tops.toSet(), hasLength(order.length));
    });

    testWidgets('the thread takes the space the other slots leave', (
      tester,
    ) async {
      await _pump(
        tester,
        ChatThreadLayout(
          banners: [_box('error')],
          thread: Container(key: const Key('thread')),
          aboveComposer: [_box('reconnect')],
          composer: _box('composer'),
        ),
      );

      final body = tester.getSize(find.byType(Scaffold)).height;
      expect(tester.getSize(find.byKey(const Key('thread'))).height, body - 60);
      expect(tester.getBottomLeft(find.byKey(const Key('composer'))).dy, body);
    });

    testWidgets('empty slot lists render nothing', (tester) async {
      await _pump(
        tester,
        ChatThreadLayout(
          banners: const [],
          thread: _box('thread'),
          aboveComposer: const [],
          composer: _box('composer'),
        ),
      );

      final body = tester.getSize(find.byType(Scaffold)).height;
      expect(tester.getTopLeft(find.byKey(const Key('thread'))).dy, 0);
      expect(tester.getSize(find.byKey(const Key('thread'))).height, body - 20);
    });
  });
}
