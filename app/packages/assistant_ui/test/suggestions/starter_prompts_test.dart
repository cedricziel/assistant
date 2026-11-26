import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Widget _host(Widget child, {double width = 400}) => MaterialApp(
  home: Scaffold(
    body: Align(
      alignment: Alignment.topLeft,
      child: SizedBox(width: width, child: child),
    ),
  ),
);

void main() {
  const prompts = ['Plan my week', 'Review a PR', 'Summarise the news'];

  Future<void> pumpAt(
    WidgetTester tester,
    double width,
    ValueChanged<String> onSelected, {
    List<String> items = prompts,
  }) async {
    tester.view
      ..physicalSize = Size(width, 800)
      ..devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(
        StarterPrompts(prompts: items, onSelected: onSelected),
        width: width,
      ),
    );
  }

  testWidgets('renders one keyed card per prompt', (tester) async {
    await pumpAt(tester, 400, (_) {});

    for (var i = 0; i < prompts.length; i++) {
      expect(find.byKey(Key('starter_prompt_$i')), findsOneWidget);
      expect(find.text(prompts[i]), findsOneWidget);
    }
  });

  testWidgets('a tap emits the prompt', (tester) async {
    final selected = <String>[];
    await pumpAt(tester, 400, selected.add);

    await tester.tap(find.byKey(const Key('starter_prompt_2')));

    expect(selected, ['Summarise the news']);
  });

  testWidgets('uses one column at 400 dp', (tester) async {
    await pumpAt(tester, 400, (_) {});

    final xs = {
      for (var i = 0; i < prompts.length; i++)
        tester.getTopLeft(find.byKey(Key('starter_prompt_$i'))).dx,
    };
    expect(xs, hasLength(1), reason: 'every card starts at the same x');
  });

  testWidgets('uses two columns at 720 dp', (tester) async {
    await pumpAt(tester, 720, (_) {});

    final x0 = tester.getTopLeft(find.byKey(const Key('starter_prompt_0'))).dx;
    final x1 = tester.getTopLeft(find.byKey(const Key('starter_prompt_1'))).dx;
    final x2 = tester.getTopLeft(find.byKey(const Key('starter_prompt_2'))).dx;
    expect(x1, greaterThan(x0), reason: 'second card sits right of the first');
    expect(x2, x0, reason: 'third card wraps to the first column');
  });

  testWidgets('an empty list renders nothing', (tester) async {
    await pumpAt(tester, 400, (_) {}, items: const []);

    expect(tester.getSize(find.byType(StarterPrompts)).height, 0);
    expect(find.byType(Card), findsNothing);
  });
}
