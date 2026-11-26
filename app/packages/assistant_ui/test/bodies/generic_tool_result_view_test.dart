import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(WidgetTester tester, ToolCallRecord record) =>
    tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: GenericToolResultView(record: record)),
      ),
    );

void main() {
  testWidgets('renders arguments as indented JSON and the result', (
    tester,
  ) async {
    await _pump(
      tester,
      ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.ok,
        arguments: {'command': 'ls'},
        result: 'a.txt',
      ),
    );

    expect(find.text('Arguments'), findsOneWidget);
    expect(find.text('{\n  "command": "ls"\n}'), findsOneWidget);
    expect(find.text('Result'), findsOneWidget);
    expect(find.text('a.txt'), findsOneWidget);
  });

  testWidgets('omits the sections a record does not have', (tester) async {
    await _pump(
      tester,
      ToolCallRecord(toolName: 'bash', status: ToolCallStatus.pending),
    );

    expect(find.text('Arguments'), findsNothing);
    expect(find.text('Result'), findsNothing);
  });

  testWidgets('shows an error result in the error colour', (tester) async {
    await _pump(
      tester,
      ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.error,
        result: 'boom',
      ),
    );

    final scheme = Theme.of(tester.element(find.byType(GenericToolResultView)))
        .colorScheme;
    expect(
      tester.widget<SelectableText>(find.byType(SelectableText)).style!.color,
      scheme.error,
    );
  });
}
