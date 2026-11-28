import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary, UserSummary;
import 'package:assistant_ui/assistant_ui.dart' show PermissionRuleTile;
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/account/account_provider.dart';
import 'package:assistant_app/features/admin/admin_providers.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/settings/tool_approvals_section.dart';
import 'package:assistant_app/features/settings/tool_permission_rules_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';

ToolPermissionRuleSummary _rule(
  String id,
  String effect,
  String rule, {
  String userId = 'u-1',
}) => ToolPermissionRuleSummary(
  (b) => b
    ..id = id
    ..userId = userId
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

class _StubSpaceNotifier extends SpaceToolPermissionRulesNotifier {
  _StubSpaceNotifier(this._load);

  final Future<List<ToolPermissionRuleSummary>> Function() _load;
  final List<String> revoked = [];
  int buildCalls = 0;

  @override
  Future<List<ToolPermissionRuleSummary>> build() {
    buildCalls++;
    return _load();
  }

  @override
  Future<void> revoke(String id) async {
    revoked.add(id);
    state = AsyncData([...state.requireValue.where((r) => r.id != id)]);
  }
}

class _Selection extends SpaceSelectionNotifier {
  @override
  SpaceSelection build() =>
      const SpaceSelection(orgId: 'org-1', spaceId: 's-1');
}

class _Users extends AdminUsersNotifier {
  _Users(this.users);

  final List<UserSummary> users;

  @override
  Future<List<UserSummary>> build() async => users;
}

UserSummary _user(String id, String name) => UserSummary(
  (b) => b
    ..id = id
    ..name = name
    ..email = '$id@example.com',
);

Future<_StubNotifier> _pump(
  WidgetTester tester,
  Future<List<ToolPermissionRuleSummary>> Function() load, {
  _StubSpaceNotifier? spaceStub,
  String role = 'member',
  List<UserSummary> users = const [],
}) async {
  final stub = _StubNotifier(load);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        toolPermissionRulesProvider.overrideWith(() => stub),
        if (spaceStub != null)
          spaceToolPermissionRulesProvider.overrideWith(() => spaceStub),
        spaceSelectionProvider.overrideWith(_Selection.new),
        callerRolesProvider.overrideWith(
          (ref) async => CallerRoles(spaces: const [], roles: {'s-1': role}),
        ),
        currentUserIdProvider.overrideWith((ref) => 'u-1'),
        adminUsersProvider.overrideWith(() => _Users(users)),
      ],
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

  group("members' approvals", () {
    testWidgets('an admin sees them below their own rules, labelled by owner', (
      tester,
    ) async {
      final spaceStub = _StubSpaceNotifier(
        () async => [
          _rule('r-1', 'allow', 'bash(own*)'),
          _rule('r-2', 'allow', 'bash(deploy*)', userId: 'u-2'),
          _rule('r-3', 'deny', 'bash(rm *)', userId: 'u-3'),
        ],
      );
      await _pump(
        tester,
        () async => [_rule('r-1', 'allow', 'bash(own*)')],
        spaceStub: spaceStub,
        role: 'space_admin',
        users: [_user('u-2', 'Ada')],
      );
      await tester.pumpAndSettle();

      expect(find.text("Members' approvals"), findsOneWidget);
      expect(
        tester.getTopLeft(find.text("Members' approvals")).dy,
        greaterThan(tester.getTopLeft(find.text('bash(own*)')).dy),
      );
      // The caller's own rule is not repeated in the members group.
      expect(find.text('bash(own*)'), findsOneWidget);
      expect(find.text('bash(deploy*)'), findsOneWidget);
      expect(find.text('Ada'), findsOneWidget);
      // Without a known member name the user id labels the rule.
      expect(find.text('u-3'), findsOneWidget);
    });

    testWidgets('a member neither requests nor sees the group', (tester) async {
      final spaceStub = _StubSpaceNotifier(
        () async => [_rule('r-2', 'allow', 'bash(deploy*)', userId: 'u-2')],
      );
      await _pump(
        tester,
        () async => [_rule('r-1', 'allow', 'bash(own*)')],
        spaceStub: spaceStub,
      );
      await tester.pumpAndSettle();

      expect(find.text("Members' approvals"), findsNothing);
      expect(find.text('bash(deploy*)'), findsNothing);
      expect(spaceStub.buildCalls, 0);
    });

    testWidgets('a viewer neither requests nor sees the group', (tester) async {
      final spaceStub = _StubSpaceNotifier(
        () async => [_rule('r-2', 'allow', 'bash(deploy*)', userId: 'u-2')],
      );
      await _pump(
        tester,
        () async => [_rule('r-1', 'allow', 'bash(own*)')],
        spaceStub: spaceStub,
        role: 'viewer',
      );
      await tester.pumpAndSettle();

      expect(find.text("Members' approvals"), findsNothing);
      expect(spaceStub.buildCalls, 0);
    });

    testWidgets('a 403 hides the group', (tester) async {
      final spaceStub = _StubSpaceNotifier(
        () async => throw DioException(
          requestOptions: RequestOptions(path: '/api/tool-permission-rules'),
          response: Response<void>(
            requestOptions: RequestOptions(path: '/api/tool-permission-rules'),
            statusCode: 403,
          ),
        ),
      );
      await _pump(
        tester,
        () async => [_rule('r-1', 'allow', 'bash(own*)')],
        spaceStub: spaceStub,
        role: 'space_admin',
      );
      await tester.pumpAndSettle();

      expect(find.text("Members' approvals"), findsNothing);
      expect(find.byType(PermissionRuleTile), findsOneWidget);
    });

    testWidgets('revoking a member rule confirms, then calls the provider', (
      tester,
    ) async {
      final spaceStub = _StubSpaceNotifier(
        () async => [_rule('r-2', 'allow', 'bash(deploy*)', userId: 'u-2')],
      );
      await _pump(
        tester,
        () async => [_rule('r-1', 'allow', 'bash(own*)')],
        spaceStub: spaceStub,
        role: 'org_admin',
        users: [_user('u-2', 'Ada')],
      );
      await tester.pumpAndSettle();

      final revoke = find.descendant(
        of: find.byKey(const ValueKey('member-r-2')),
        matching: find.byTooltip('Revoke'),
      );
      await tester.ensureVisible(revoke);
      await tester.tap(revoke);
      await tester.pumpAndSettle();

      expect(find.text('Revoke approval?'), findsOneWidget);
      expect(spaceStub.revoked, isEmpty);

      await tester.tap(find.text('Revoke').last);
      await tester.pumpAndSettle();

      expect(spaceStub.revoked, ['r-2']);
      expect(find.text('bash(deploy*)'), findsNothing);
      expect(find.text("Members' approvals"), findsNothing);
    });
  });
}
