import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _lisbon = [
  SearchSegmentItem(text: 'The ', highlighted: false),
  SearchSegmentItem(text: 'Lisbon', highlighted: true),
  SearchSegmentItem(text: ' itinerary is ready', highlighted: false),
];

/// Leaf spans of the first [RichText] under [find], in order.
List<TextSpan> _leaves(WidgetTester tester, Finder find) {
  final root = tester.widget<RichText>(find).text;
  final leaves = <TextSpan>[];
  root.visitChildren((span) {
    if (span is TextSpan && span.text != null) leaves.add(span);
    return true;
  });
  return leaves;
}

Widget _host(Widget child, {ThemeData? theme}) => MaterialApp(
  theme: theme,
  home: Scaffold(body: child),
);

void main() {
  testWidgets('builds one span per segment, in order', (tester) async {
    await tester.pumpWidget(_host(const HighlightedSegments(_lisbon)));

    final leaves = _leaves(tester, find.byType(RichText).last);
    expect(leaves.map((s) => s.text), [
      'The ',
      'Lisbon',
      ' itinerary is ready',
    ]);
  });

  testWidgets('highlighted spans are bold and use the primary colour', (
    tester,
  ) async {
    final theme = ThemeData(colorScheme: const ColorScheme.light());
    await tester.pumpWidget(
      _host(const HighlightedSegments(_lisbon), theme: theme),
    );

    final leaves = _leaves(tester, find.byType(RichText).last);
    expect(leaves[1].style?.fontWeight, FontWeight.bold);
    expect(leaves[1].style?.color, theme.colorScheme.primary);
    expect(leaves[0].style?.fontWeight, isNot(FontWeight.bold));
    expect(leaves[0].style?.color, isNull);
    expect(leaves[2].style?.fontWeight, isNot(FontWeight.bold));
  });

  testWidgets('the base style applies to every span', (tester) async {
    await tester.pumpWidget(
      _host(
        const HighlightedSegments(
          _lisbon,
          style: TextStyle(fontSize: 11, fontStyle: FontStyle.italic),
        ),
      ),
    );

    final richText = tester.widget<RichText>(find.byType(RichText).last);
    expect(richText.text.style?.fontSize, 11);
    expect(richText.text.style?.fontStyle, FontStyle.italic);
  });

  testWidgets('reads as one plain string for screen readers', (tester) async {
    await tester.pumpWidget(_host(const HighlightedSegments(_lisbon)));

    expect(find.bySemanticsLabel('The Lisbon itinerary is ready'), findsOne);
  });

  testWidgets('no segments renders nothing', (tester) async {
    await tester.pumpWidget(_host(const HighlightedSegments([])));

    expect(
      find.descendant(
        of: find.byType(HighlightedSegments),
        matching: find.byType(RichText),
      ),
      findsNothing,
    );
  });
}
