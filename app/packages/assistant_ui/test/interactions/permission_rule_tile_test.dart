import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _created = DateTime.utc(2026, 1, 15, 12);

Future<void> _pump(
  WidgetTester tester, {
  PermissionRuleEffect effect = PermissionRuleEffect.allow,
  String rule = 'bash(git status*)',
  String? ownerLabel,
  bool revoking = false,
  VoidCallback? onRevoke,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: PermissionRuleTile(
        effect: effect,
        rule: rule,
        createdAt: _created,
        ownerLabel: ownerLabel,
        revoking: revoking,
        onRevoke: onRevoke ?? () {},
      ),
    ),
  ),
);

void main() {
  testWidgets('shows the allow badge, the rule and the creation date', (
    tester,
  ) async {
    await _pump(tester);

    expect(find.text('Allow'), findsOneWidget);
    expect(find.text('bash(git status*)'), findsOneWidget);
    expect(find.text('Saved Jan 15, 2026'), findsOneWidget);
  });

  testWidgets('shows the deny badge for a deny rule', (tester) async {
    await _pump(tester, effect: PermissionRuleEffect.deny);

    expect(find.text('Deny'), findsOneWidget);
    expect(find.text('Allow'), findsNothing);
  });

  testWidgets('tapping revoke fires the callback once', (tester) async {
    var revoked = 0;
    await _pump(tester, onRevoke: () => revoked++);

    await tester.tap(find.byTooltip('Revoke'));

    expect(revoked, 1);
  });

  testWidgets('no owner label by default', (tester) async {
    await _pump(tester);

    expect(find.byKey(const Key('permission_rule_owner')), findsNothing);
  });

  testWidgets('renders the owner label and revoke still fires', (tester) async {
    var revoked = 0;
    await _pump(
      tester,
      ownerLabel: 'ada@example.com',
      onRevoke: () => revoked++,
    );

    expect(find.text('ada@example.com'), findsOneWidget);

    await tester.tap(find.byTooltip('Revoke'));

    expect(revoked, 1);
  });

  testWidgets('revoke is disabled while revoking', (tester) async {
    var revoked = 0;
    await _pump(tester, revoking: true, onRevoke: () => revoked++);

    await tester.tap(find.byTooltip('Revoke'), warnIfMissed: false);

    expect(revoked, 0);
  });

  testWidgets('a long rule is ellipsised, not overflowed', (tester) async {
    await _pump(tester, rule: 'bash(${'very-long-argument ' * 20}*)');

    expect(tester.takeException(), isNull);
  });
}
