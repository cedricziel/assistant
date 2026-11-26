import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary;
import 'package:assistant_ui/assistant_ui.dart' show PermissionRuleTile;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/settings/tool_approvals_section.dart';
import 'package:assistant_app/features/settings/tool_permission_rules_provider.dart';

ToolPermissionRuleSummary _rule(String id, String effect, String rule) =>
    ToolPermissionRuleSummary(
      (b) => b
        ..id = id
        ..userId = 'u-1'
        ..effect = effect
        ..rule = rule
        ..toolName = rule.split('(').first
        ..createdAt = '2026-01-02T03:04:05Z',
    );

class _StubNotifier extends ToolPermissionRulesNotifier {
  _StubNotifier(this._load);

  final Future<List<ToolPermissionRuleSummary>> Function() _load;
  final List<String> revoked = [];
  Object? revokeError;

  @override
  Future<List<ToolPermissionRuleSummary>> build() => _load();

  @override
  Future<void> revoke(String id) async {
    revoked.add(id);
    if (revokeError != null) throw revokeError!;
    state = AsyncData([...state.requireValue.where((r) => r.id != id)]);
  }
}

Future<_StubNotifier> _pump(
  WidgetTester tester,
  Future<List<ToolPermissionRuleSummary>> Function() load,
) async {
  final stub = _StubNotifier(load);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [toolPermissionRulesProvider.overrideWith(() => stub)],
      child: const MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(child: ToolApprovalsSection()),
        ),
      ),
    ),
  );
  return stub;
}

void main() {
  const note =
      'Saved approvals apply to all of your turns in this space, whichever '
      'chat or interface they run from.';

  testWidgets('shows a spinner while loading', (tester) async {
    await _pump(
      tester,
      () => Future.delayed(const Duration(days: 1), () => []),
    );
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text('Tool approvals'), findsOneWidget);
    // Drain the pending timer.
    await tester.pump(const Duration(days: 2));
  });

  testWidgets('shows the error and retries', (tester) async {
    var calls = 0;
    await _pump(tester, () async {
      calls++;
      if (calls == 1) throw Exception('nope');
      return [_rule('r-1', 'allow', 'bash(git status*)')];
    });
    await tester.pumpAndSettle();

    expect(
      find.textContaining('Could not load saved approvals'),
      findsOneWidget,
    );

    await tester.tap(find.text('Retry'));
    await tester.pumpAndSettle();

    expect(find.byType(PermissionRuleTile), findsOneWidget);
  });

  testWidgets('explains the empty state', (tester) async {
    await _pump(tester, () async => []);
    await tester.pumpAndSettle();

    expect(find.text('No approvals saved'), findsOneWidget);
    expect(find.textContaining('Always allow'), findsOneWidget);
    expect(find.byType(PermissionRuleTile), findsNothing);
  });

  testWidgets('groups rules under Allowed and Denied with the scope note', (
    tester,
  ) async {
    await _pump(
      tester,
      () async => [
        _rule('r-1', 'allow', 'bash(git status*)'),
        _rule('r-2', 'deny', 'bash(rm *)'),
        _rule('r-3', 'allow', 'web-fetch(https://docs.rs/*)'),
      ],
    );
    await tester.pumpAndSettle();

    expect(find.text(note), findsOneWidget);
    expect(find.text('Allowed'), findsOneWidget);
    expect(find.text('Denied'), findsOneWidget);
    expect(find.byType(PermissionRuleTile), findsNWidgets(3));
    expect(find.text('bash(git status*)'), findsOneWidget);
    expect(find.text('bash(rm *)'), findsOneWidget);
    // Allowed rules come before the Denied heading.
    expect(
      tester.getTopLeft(find.text('web-fetch(https://docs.rs/*)')).dy,
      lessThan(tester.getTopLeft(find.text('Denied')).dy),
    );
  });

  testWidgets('omits a group that has no rules', (tester) async {
    await _pump(tester, () async => [_rule('r-1', 'allow', 'bash(ls*)')]);
    await tester.pumpAndSettle();

    expect(find.text('Allowed'), findsOneWidget);
    expect(find.text('Denied'), findsNothing);
  });

  testWidgets('revoke asks for confirmation, then calls the provider', (
    tester,
  ) async {
    final stub = await _pump(
      tester,
      () async => [_rule('r-1', 'allow', 'bash(git status*)')],
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Revoke'));
    await tester.pumpAndSettle();

    expect(find.text('Revoke approval?'), findsOneWidget);
    expect(stub.revoked, isEmpty);

    await tester.tap(find.text('Revoke').last);
    await tester.pumpAndSettle();

    expect(stub.revoked, ['r-1']);
    expect(find.byType(PermissionRuleTile), findsNothing);
    expect(find.text('No approvals saved'), findsOneWidget);
  });

  testWidgets('cancelling the confirmation revokes nothing', (tester) async {
    final stub = await _pump(
      tester,
      () async => [_rule('r-1', 'allow', 'bash(git status*)')],
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Revoke'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(stub.revoked, isEmpty);
    expect(find.byType(PermissionRuleTile), findsOneWidget);
  });

  testWidgets('a failed revoke is reported and keeps the row', (tester) async {
    final stub = await _pump(
      tester,
      () async => [_rule('r-1', 'allow', 'bash(git status*)')],
    );
    stub.revokeError = Exception('server down');
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Revoke'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Revoke').last);
    await tester.pumpAndSettle();

    expect(find.textContaining('Could not revoke'), findsOneWidget);
    expect(find.byType(PermissionRuleTile), findsOneWidget);
  });
}
