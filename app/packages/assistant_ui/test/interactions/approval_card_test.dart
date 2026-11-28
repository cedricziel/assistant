import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

const _allOptions = [
  ApprovalDecision.allowOnce,
  ApprovalDecision.allowAlways,
  ApprovalDecision.rejectOnce,
  ApprovalDecision.rejectAlways,
];

InteractionRecord _record({
  InteractionStatus status = InteractionStatus.pending,
  List<ApprovalDecision> options = _allOptions,
  String? suggestedRule = 'bash(git status*)',
  ApprovalDecision? decision,
  bool argumentsTruncated = false,
  Map<String, Object?> arguments = const {'command': 'git status --short'},
  DateTime? expiresAt,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.toolApproval,
  status: status,
  createdAt: _now,
  expiresAt: expiresAt ?? _now.add(const Duration(minutes: 9, seconds: 42)),
  toolName: 'bash',
  arguments: arguments,
  argumentsTruncated: argumentsTruncated,
  reason: "no rule allows 'bash'",
  options: options,
  suggestedRule: suggestedRule,
  decision: decision,
);

Future<void> _pump(
  WidgetTester tester,
  InteractionRecord record, {
  ValueChanged<ApprovalDecision>? onDecision,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        child: ApprovalCard(
          record: record,
          now: _now,
          onDecision: onDecision ?? (_) {},
        ),
      ),
    ),
  ),
);

void main() {
  group('models', () {
    test('InteractionKind.fromWire maps known kinds and falls back', () {
      expect(
        InteractionKind.fromWire('tool_approval'),
        InteractionKind.toolApproval,
      );
      expect(InteractionKind.fromWire('question'), InteractionKind.question);
      expect(
        InteractionKind.fromWire('surface_action'),
        InteractionKind.surfaceAction,
      );
      expect(InteractionKind.fromWire('nope'), InteractionKind.unknown);
    });

    test('ApprovalDecision round-trips its wire name', () {
      for (final d in ApprovalDecision.values) {
        expect(ApprovalDecision.fromWire(d.wire), d);
      }
      expect(ApprovalDecision.fromWire('nope'), isNull);
    });

    test('rendersCard defaults to true', () {
      expect(_record().rendersCard, isTrue);
    });

    test('TimelineEntryType has the interaction fallback', () {
      expect(TimelineEntryType.values, contains(TimelineEntryType.interaction));
    });
  });

  group('pending', () {
    testWidgets('shows tool, summary, reason and countdown', (tester) async {
      await _pump(tester, _record());

      expect(find.text('bash'), findsOneWidget);
      expect(find.text('git status --short'), findsOneWidget);
      expect(find.textContaining("no rule allows 'bash'"), findsOneWidget);
      expect(find.text('Expires in 9:42'), findsOneWidget);
    });

    testWidgets('shows only the offered buttons', (tester) async {
      await _pump(
        tester,
        _record(
          options: const [
            ApprovalDecision.allowOnce,
            ApprovalDecision.rejectOnce,
          ],
          suggestedRule: null,
        ),
      );

      expect(find.byKey(const Key('approval_allow_once')), findsOneWidget);
      expect(find.byKey(const Key('approval_reject_once')), findsOneWidget);
      expect(find.byKey(const Key('approval_allow_always')), findsNothing);
      expect(find.byKey(const Key('approval_reject_always')), findsNothing);
      expect(find.textContaining('applies to all your turns'), findsNothing);
    });

    testWidgets('always buttons show the rule and the scope note', (
      tester,
    ) async {
      await _pump(tester, _record());

      expect(find.byKey(const Key('approval_allow_always')), findsOneWidget);
      expect(find.byKey(const Key('approval_reject_always')), findsOneWidget);
      expect(find.text('bash(git status*)'), findsOneWidget);
      expect(
        find.textContaining(
          'applies to all your turns in this space, including scheduled '
          'tasks and messenger chats',
        ),
        findsOneWidget,
      );
    });

    testWidgets('arguments are collapsed until expanded', (tester) async {
      await _pump(
        tester,
        _record(
          arguments: const {'command': 'git status --short', 'cwd': '/x'},
        ),
      );

      expect(find.textContaining('"cwd"'), findsNothing);
      await tester.tap(find.byKey(const Key('approval_arguments_toggle')));
      await tester.pump();
      expect(find.textContaining('"cwd": "/x"'), findsOneWidget);
    });

    testWidgets('truncated arguments say so when expanded', (tester) async {
      await _pump(tester, _record(argumentsTruncated: true));

      expect(find.textContaining('shortened'), findsNothing);
      await tester.tap(find.byKey(const Key('approval_arguments_toggle')));
      await tester.pump();
      expect(find.textContaining('shortened'), findsOneWidget);
    });

    testWidgets('a tap fires onDecision with the matching enum', (
      tester,
    ) async {
      final fired = <ApprovalDecision>[];
      await _pump(tester, _record(), onDecision: fired.add);

      for (final d in _allOptions) {
        await tester.tap(find.byKey(Key('approval_${d.wire}')));
      }

      expect(fired, _allOptions);
    });

    testWidgets('an overdue card says it is expiring', (tester) async {
      await _pump(tester, _record(expiresAt: _now));

      expect(find.text('Expiring…'), findsOneWidget);
    });

    testWidgets('semantics labels name the action and the tool', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, _record());

      for (final label in [
        'Allow bash once',
        'Always allow bash',
        'Reject bash once',
        'Always reject bash',
      ]) {
        final node = tester.getSemantics(find.bySemanticsLabel(label));
        expect(node.label, label);
        expect(node.flagsCollection.isButton, isTrue, reason: label);
        expect(node.flagsCollection.isEnabled, Tristate.isTrue, reason: label);
      }
      handle.dispose();
    });
  });

  group('submitting', () {
    testWidgets('disables every button', (tester) async {
      final fired = <ApprovalDecision>[];
      await _pump(
        tester,
        _record(status: InteractionStatus.submitting),
        onDecision: fired.add,
      );

      for (final d in _allOptions) {
        final button = tester.widget<ButtonStyleButton>(
          find.byKey(Key('approval_${d.wire}')),
        );
        expect(button.onPressed, isNull, reason: d.wire);
      }
      await tester.tap(find.byKey(const Key('approval_allow_once')));
      expect(fired, isEmpty);
    });
  });

  group('receipts', () {
    final cases = <String, InteractionRecord>{
      'Allowed once': _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowOnce,
      ),
      'Always allowed · bash(git status*)': _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowAlways,
      ),
      'Rejected once': _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.rejectOnce,
      ),
      'Always rejected · bash(git status*)': _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.rejectAlways,
      ),
      'Expired without an answer': _record(status: InteractionStatus.expired),
      'Cancelled': _record(status: InteractionStatus.cancelled),
    };

    for (final entry in cases.entries) {
      testWidgets('${entry.key} renders with no buttons', (tester) async {
        await _pump(tester, entry.value);

        expect(find.text(entry.key), findsOneWidget);
        expect(find.byType(ButtonStyleButton), findsNothing);
        expect(find.text('Expires in 9:42'), findsNothing);
      });
    }
  });

  group('keyboard', () {
    testWidgets('Enter on the focused card fires allow once', (tester) async {
      final fired = <ApprovalDecision>[];
      await _pump(tester, _record(), onDecision: fired.add);

      Focus.of(tester.element(find.byKey(const Key('approval_card_focus'))))
          .requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);

      expect(fired, [ApprovalDecision.allowOnce]);
    });

    testWidgets('Escape fires nothing', (tester) async {
      final fired = <ApprovalDecision>[];
      await _pump(tester, _record(), onDecision: fired.add);

      Focus.of(tester.element(find.byKey(const Key('approval_card_focus'))))
          .requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);

      expect(fired, isEmpty);
    });

    testWidgets('Enter on a focused button does not fire allow once', (
      tester,
    ) async {
      final fired = <ApprovalDecision>[];
      await _pump(tester, _record(), onDecision: fired.add);

      Focus.of(
        tester.element(
          find.descendant(
            of: find.byKey(const Key('approval_reject_once')),
            matching: find.text('Reject once'),
          ),
        ),
      ).requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);

      expect(fired, [ApprovalDecision.rejectOnce]);
    });

    testWidgets('Enter does nothing while submitting', (tester) async {
      final fired = <ApprovalDecision>[];
      await _pump(
        tester,
        _record(status: InteractionStatus.submitting),
        onDecision: fired.add,
      );

      Focus.of(tester.element(find.byKey(const Key('approval_card_focus'))))
          .requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);

      expect(fired, isEmpty);
    });
  });
}
