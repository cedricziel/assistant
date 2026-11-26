import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(
  WidgetTester tester,
  Map<String, Object?> data, {
  int initiallyExpandedDepth = 1,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        child: JsonTreeView(
          data: data,
          initiallyExpandedDepth: initiallyExpandedDepth,
        ),
      ),
    ),
  ),
);

void main() {
  testWidgets('shows top-level keys with scalar values', (tester) async {
    await _pump(tester, {'truncated': false, 'count': 3, 'name': 'a.txt'});

    expect(find.text('truncated'), findsOneWidget);
    expect(find.text('false'), findsOneWidget);
    expect(find.text('count'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('"a.txt"'), findsOneWidget);
  });

  testWidgets('shows null values', (tester) async {
    await _pump(tester, {'next': null});

    expect(find.text('next'), findsOneWidget);
    expect(find.text('null'), findsOneWidget);
  });

  testWidgets('nested containers start collapsed and show their size', (
    tester,
  ) async {
    await _pump(tester, {
      'paths': ['a.txt', 'b.txt'],
      'meta': {'k': 1},
    });

    expect(find.text('paths'), findsOneWidget);
    expect(find.text('[2]'), findsOneWidget);
    expect(find.text('meta'), findsOneWidget);
    expect(find.text('{1}'), findsOneWidget);
    expect(find.text('"a.txt"'), findsNothing);
    expect(find.text('k'), findsNothing);
  });

  testWidgets('tapping a container expands and collapses it', (tester) async {
    await _pump(tester, {
      'paths': ['a.txt', 'b.txt'],
    });

    await tester.tap(find.text('paths'));
    await tester.pump();

    expect(find.text('0'), findsOneWidget);
    expect(find.text('"a.txt"'), findsOneWidget);
    expect(find.text('"b.txt"'), findsOneWidget);

    await tester.tap(find.text('paths'));
    await tester.pump();

    expect(find.text('"a.txt"'), findsNothing);
  });

  testWidgets('initiallyExpandedDepth opens nested containers', (tester) async {
    await _pump(tester, {
      'meta': {
        'inner': {'deep': true},
      },
    }, initiallyExpandedDepth: 3);

    expect(find.text('inner'), findsOneWidget);
    expect(find.text('deep'), findsOneWidget);
    expect(find.text('true'), findsOneWidget);
  });

  testWidgets('an empty map renders as {}', (tester) async {
    await _pump(tester, const {});

    expect(find.text('{}'), findsOneWidget);
  });

  testWidgets('container rows are expandable buttons for assistive tech', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await _pump(tester, {
      'paths': ['a'],
    });

    final node = tester.getSemantics(find.bySemanticsLabel('paths [1]'));
    expect(node.flagsCollection.isButton, isTrue);
    expect(node.getSemanticsData().hasAction(SemanticsAction.tap), isTrue);
    expect(node.flagsCollection.isExpanded, Tristate.isFalse);

    await tester.tap(find.text('paths'));
    await tester.pump();

    expect(
      tester
          .getSemantics(find.bySemanticsLabel('paths [1]'))
          .flagsCollection
          .isExpanded,
      Tristate.isTrue,
    );
    handle.dispose();
  });
}
