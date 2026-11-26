import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _host(Widget child) => MaterialApp(
  home: Scaffold(body: Center(child: child)),
);

void main() {
  const prompts = ['Summarise this', 'Draft a reply', 'Explain in detail'];

  testWidgets('renders one keyed chip per prompt', (tester) async {
    await tester.pumpWidget(
      _host(SuggestionChips(prompts: prompts, onSelected: (_) {})),
    );

    for (var i = 0; i < prompts.length; i++) {
      expect(find.byKey(Key('suggestion_chip_$i')), findsOneWidget);
      expect(find.text(prompts[i]), findsOneWidget);
    }
  });

  testWidgets('a tap fires onSelected with the prompt', (tester) async {
    final selected = <String>[];
    await tester.pumpWidget(
      _host(SuggestionChips(prompts: prompts, onSelected: selected.add)),
    );

    await tester.tap(find.byKey(const Key('suggestion_chip_1')));

    expect(selected, ['Draft a reply']);
  });

  testWidgets('enabled: false blocks taps', (tester) async {
    final selected = <String>[];
    await tester.pumpWidget(
      _host(
        SuggestionChips(
          prompts: prompts,
          onSelected: selected.add,
          enabled: false,
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('suggestion_chip_0')));

    expect(selected, isEmpty);
  });

  testWidgets('an empty list renders SizedBox.shrink', (tester) async {
    await tester.pumpWidget(
      _host(SuggestionChips(prompts: const [], onSelected: (_) {})),
    );

    expect(tester.getSize(find.byType(SuggestionChips)), Size.zero);
    expect(find.byType(ActionChip), findsNothing);
  });

  testWidgets('the group is labelled and each chip is a button', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      _host(SuggestionChips(prompts: prompts, onSelected: (_) {})),
    );

    expect(find.bySemanticsLabel('Suggested follow-ups'), findsOneWidget);
    for (var i = 0; i < prompts.length; i++) {
      final data = tester
          .getSemantics(find.byKey(Key('suggestion_chip_$i')))
          .getSemanticsData();
      expect(
        data.flagsCollection.isButton,
        isTrue,
        reason: 'chip $i exposes button semantics',
      );
    }
    handle.dispose();
  });

  testWidgets('chips meet the Android tap-target guideline', (tester) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      _host(SuggestionChips(prompts: prompts, onSelected: (_) {})),
    );

    await expectLater(tester, meetsGuideline(androidTapTargetGuideline));
    handle.dispose();
  });

  testWidgets('a long prompt stays inside the available width', (tester) async {
    tester.view
      ..physicalSize = const Size(320, 600)
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(
        SuggestionChips(
          prompts: [List.filled(30, 'extremely').join(' ')],
          onSelected: (_) {},
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byKey(const Key('suggestion_chip_0'))).width,
      lessThanOrEqualTo(320),
    );
  });
}
