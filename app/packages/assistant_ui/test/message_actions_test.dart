import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _wrap(Widget child) => MaterialApp(
  home: Scaffold(
    body: Align(alignment: Alignment.topLeft, child: child),
  ),
);

Color _iconColor(WidgetTester tester, Key key) => tester
    .widget<Icon>(
      find.descendant(of: find.byKey(key), matching: find.byType(Icon)),
    )
    .color!;

Color _labelColor(WidgetTester tester, Key key) => tester
    .widget<Text>(
      find.descendant(of: find.byKey(key), matching: find.byType(Text)),
    )
    .style!
    .color!;

void main() {
  group('MessageActions', () {
    testWidgets('lays out leading, actions, then trailing in order', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          MessageActions(
            leading: const Text('L', key: Key('leading')),
            actions: [
              MessageActionButton(
                key: const Key('a1'),
                icon: Icons.copy,
                label: 'One',
                onPressed: () {},
              ),
              MessageActionButton(
                key: const Key('a2'),
                icon: Icons.share,
                label: 'Two',
                onPressed: () {},
              ),
            ],
            trailing: const [
              Text('T1', key: Key('t1')),
              Text('T2', key: Key('t2')),
            ],
          ),
        ),
      );

      final keys = ['leading', 'a1', 'a2', 't1', 't2'];
      final lefts = [
        for (final k in keys) tester.getTopLeft(find.byKey(Key(k))).dx,
      ];
      expect(lefts, [...lefts]..sort(), reason: 'left-to-right order');
      expect(lefts.toSet().length, keys.length, reason: 'distinct positions');
    });

    testWidgets('renders nothing when everything is empty', (tester) async {
      await tester.pumpWidget(_wrap(const MessageActions(actions: [])));

      expect(tester.getSize(find.byType(MessageActions)), Size.zero);
    });

    testWidgets('aligns to the end when asked', (tester) async {
      await tester.pumpWidget(
        _wrap(
          SizedBox(
            width: 300,
            child: MessageActions(
              alignment: WrapAlignment.end,
              actions: [
                MessageActionButton(
                  key: const Key('a'),
                  icon: Icons.copy,
                  label: 'Copy',
                  onPressed: () {},
                ),
              ],
            ),
          ),
        ),
      );

      expect(tester.getTopRight(find.byKey(const Key('a'))).dx, 300);
    });
  });

  group('MessageActionButton', () {
    testWidgets('its hit area is at least 24x24 (WCAG 2.2 SC 2.5.8)', (
      tester,
    ) async {
      await tester.pumpWidget(
        _wrap(
          MessageActionButton(
            key: const Key('a'),
            icon: Icons.copy,
            label: 'Copy',
            onPressed: () {},
          ),
        ),
      );

      final size = tester.getSize(find.byKey(const Key('a')));
      expect(size.width, greaterThanOrEqualTo(24));
      expect(size.height, greaterThanOrEqualTo(24));
    });

    testWidgets('fires onPressed on tap', (tester) async {
      var taps = 0;
      await tester.pumpWidget(
        _wrap(
          MessageActionButton(
            icon: Icons.copy,
            label: 'Copy',
            onPressed: () => taps++,
          ),
        ),
      );

      await tester.tap(find.text('Copy'));

      expect(taps, 1);
    });

    testWidgets('selected reports selected semantics and the selected colour', (
      tester,
    ) async {
      const key = Key('b');
      await tester.pumpWidget(
        _wrap(
          MessageActionButton(
            key: key,
            icon: Icons.thumb_up,
            label: 'Good',
            selected: true,
            onPressed: () {},
          ),
        ),
      );

      final scheme = Theme.of(tester.element(find.byKey(key))).colorScheme;
      final semantics = tester.getSemantics(find.byKey(key));
      expect(semantics.flagsCollection.isSelected, Tristate.isTrue);
      expect(semantics.label, 'Good');
      expect(_iconColor(tester, key), scheme.primary);
      expect(_labelColor(tester, key), scheme.primary);
    });

    testWidgets('unselected muted button is not selected and is muted', (
      tester,
    ) async {
      const key = Key('b');
      await tester.pumpWidget(
        _wrap(
          MessageActionButton(
            key: key,
            icon: Icons.copy,
            label: 'Copy',
            onPressed: () {},
          ),
        ),
      );

      final scheme = Theme.of(tester.element(find.byKey(key))).colorScheme;
      expect(
        tester.getSemantics(find.byKey(key)).flagsCollection.isSelected,
        isNot(Tristate.isTrue),
      );
      expect(_iconColor(tester, key), scheme.onSurfaceVariant);
    });

    testWidgets('danger tone uses the error colour', (tester) async {
      const key = Key('b');
      await tester.pumpWidget(
        _wrap(
          MessageActionButton(
            key: key,
            icon: Icons.refresh,
            label: 'Retry',
            tone: MessageActionTone.danger,
            onPressed: () {},
          ),
        ),
      );

      final scheme = Theme.of(tester.element(find.byKey(key))).colorScheme;
      expect(_iconColor(tester, key), scheme.error);
      expect(_labelColor(tester, key), scheme.error);
    });
  });
}
