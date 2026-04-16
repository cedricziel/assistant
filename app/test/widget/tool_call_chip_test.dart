import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/tool_call_chip.dart';

Widget _wrap(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  group('ToolCallChip', () {
    testWidgets('shows spinner for pending status', (tester) async {
      final record = ToolCallRecord(
        toolName: 'web-search',
        status: ToolCallStatus.pending,
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text('web-search'), findsOneWidget);
      expect(find.byIcon(Icons.check_circle_outline), findsNothing);
    });

    testWidgets('shows checkmark icon for ok status', (tester) async {
      final record = ToolCallRecord(
        toolName: 'file-read',
        status: ToolCallStatus.ok,
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byIcon(Icons.check_circle_outline), findsOneWidget);
      expect(find.text('file-read'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    });

    testWidgets('shows error icon for error status', (tester) async {
      final record = ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.error,
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byIcon(Icons.error_outline), findsOneWidget);
      expect(find.text('bash'), findsOneWidget);
    });

    testWidgets('shows block icon for denied status', (tester) async {
      final record = ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.denied,
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byIcon(Icons.block), findsOneWidget);
      expect(find.text('bash'), findsOneWidget);
    });

    testWidgets('shows expand icon when arguments are present', (tester) async {
      final record = ToolCallRecord(
        toolName: 'web-search',
        status: ToolCallStatus.ok,
        arguments: {'query': 'flutter'},
        result: 'Found 5 results.',
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);
      // Details not visible until tapped.
      expect(find.text('Arguments'), findsNothing);
    });

    testWidgets('no expand icon when no details', (tester) async {
      final record = ToolCallRecord(
        toolName: 'web-search',
        status: ToolCallStatus.ok,
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      expect(find.byIcon(Icons.expand_more_rounded), findsNothing);
    });

    testWidgets('expands to show arguments and result on tap', (tester) async {
      final record = ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.ok,
        arguments: {'command': 'ls'},
        result: 'file1.txt\nfile2.txt',
      );
      await tester.pumpWidget(_wrap(ToolCallChip(record: record)));

      // Tap to expand.
      await tester.tap(find.text('bash'));
      await tester.pumpAndSettle();

      expect(find.text('Arguments'), findsOneWidget);
      expect(find.text('Result'), findsOneWidget);
      expect(find.textContaining('file1.txt'), findsOneWidget);
      expect(find.byIcon(Icons.expand_less_rounded), findsOneWidget);

      // Tap again to collapse.
      await tester.tap(find.text('bash'));
      await tester.pumpAndSettle();

      expect(find.text('Arguments'), findsNothing);
      expect(find.byIcon(Icons.expand_more_rounded), findsOneWidget);
    });
  });
}
