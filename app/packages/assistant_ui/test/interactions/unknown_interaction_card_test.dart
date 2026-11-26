import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(WidgetTester tester) => tester.pumpWidget(
  const MaterialApp(home: Scaffold(body: UnknownInteractionCard())),
);

void main() {
  testWidgets('says the request needs a newer app', (tester) async {
    await _pump(tester);

    expect(find.textContaining('newer version of the app'), findsOneWidget);
  });

  testWidgets('is inert: no buttons or tappable controls', (tester) async {
    await _pump(tester);

    expect(find.byType(ButtonStyleButton), findsNothing);
    expect(find.byType(IconButton), findsNothing);
    expect(find.byType(InkWell), findsNothing);
    expect(find.byType(GestureDetector), findsNothing);
  });

  testWidgets('exposes the notice to assistive technology', (tester) async {
    final handle = tester.ensureSemantics();
    await _pump(tester);

    expect(
      tester.getSemantics(find.byType(UnknownInteractionCard)),
      matchesSemantics(label: 'This request needs a newer version of the app.'),
    );
    handle.dispose();
  });
}
