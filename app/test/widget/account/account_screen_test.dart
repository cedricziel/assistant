import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/account/account_provider.dart';
import 'package:assistant_app/features/account/account_screen.dart';

UserDetail _user({
  String id = 'user-1',
  String orgId = 'org-1',
  String email = 'alice@test.local',
  String name = 'Alice',
}) {
  return UserDetail(
    (b) => b
      ..id = id
      ..orgId = orgId
      ..email = email
      ..name = name
      ..createdAt = '2025-01-15T00:00:00Z'
      ..updatedAt = '2025-01-15T00:00:00Z',
  );
}

AccountState _state({UserDetail? user, bool isOidc = false, String? issuer}) {
  return AccountState(user: user ?? _user(), isOidc: isOidc, issuer: issuer);
}

void main() {
  group('AccountScreen', () {
    testWidgets(
      'renders name, email, and password sections for password-mode',
      (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              accountProvider.overrideWith(
                () => _StubAccountNotifier(_state()),
              ),
            ],
            child: const MaterialApp(home: AccountScreen()),
          ),
        );
        await tester.pumpAndSettle();

        expect(find.text('Account'), findsOneWidget);
        expect(find.text('Name'), findsOneWidget);
        expect(find.text('Email'), findsOneWidget);
        expect(find.text('Password'), findsOneWidget);
        expect(find.byKey(const Key('account.password.save')), findsOneWidget);
      },
    );

    testWidgets('shows OIDC banner and hides password section for OIDC orgs', (
      tester,
    ) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            accountProvider.overrideWith(
              () => _StubAccountNotifier(
                _state(isOidc: true, issuer: 'https://idp.example.com'),
              ),
            ),
          ],
          child: const MaterialApp(home: AccountScreen()),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.textContaining('Managed by your identity provider'),
        findsOneWidget,
      );
      expect(find.textContaining('https://idp.example.com'), findsOneWidget);
      expect(find.byKey(const Key('account.password.save')), findsNothing);
    });

    testWidgets('email confirmation mismatch blocks submit', (tester) async {
      final notifier = _StubAccountNotifier(_state());
      await tester.pumpWidget(
        ProviderScope(
          overrides: [accountProvider.overrideWith(() => notifier)],
          child: const MaterialApp(home: AccountScreen()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.enterText(
        find.byKey(const Key('account.email.new')),
        'new@example.com',
      );
      await tester.enterText(
        find.byKey(const Key('account.email.confirm')),
        'mismatch@example.com',
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('account.email.save')));
      await tester.pumpAndSettle();

      expect(find.textContaining('do not match'), findsOneWidget);
      expect(notifier.updateEmailCalls, 0);
    });

    testWidgets('successful name save triggers updateName', (tester) async {
      final notifier = _StubAccountNotifier(_state());
      await tester.pumpWidget(
        ProviderScope(
          overrides: [accountProvider.overrideWith(() => notifier)],
          child: const MaterialApp(home: AccountScreen()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.enterText(
        find.byKey(const Key('account.name.field')),
        'Alice Updated',
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('account.name.save')));
      await tester.pumpAndSettle();

      expect(notifier.updateNameCalls, 1);
      expect(notifier.lastNameValue, 'Alice Updated');
    });

    testWidgets('password mismatch blocks submit', (tester) async {
      final notifier = _StubAccountNotifier(_state());
      await tester.pumpWidget(
        ProviderScope(
          overrides: [accountProvider.overrideWith(() => notifier)],
          child: const MaterialApp(home: AccountScreen()),
        ),
      );
      await tester.pumpAndSettle();

      await tester.enterText(
        find.byKey(const Key('account.password.current')),
        'oldpw',
      );
      await tester.enterText(
        find.byKey(const Key('account.password.new')),
        'newpw',
      );
      await tester.enterText(
        find.byKey(const Key('account.password.confirm')),
        'different',
      );
      await tester.pumpAndSettle();

      final saveBtn = find.byKey(const Key('account.password.save'));
      await tester.ensureVisible(saveBtn);
      await tester.pumpAndSettle();
      await tester.tap(saveBtn);
      await tester.pumpAndSettle();

      expect(find.textContaining('do not match'), findsOneWidget);
      expect(notifier.changePasswordCalls, 0);
    });
  });
}

class _StubAccountNotifier extends AsyncNotifier<AccountState>
    implements AccountNotifier {
  _StubAccountNotifier(this._state);
  final AccountState _state;

  int updateNameCalls = 0;
  int updateEmailCalls = 0;
  int changePasswordCalls = 0;
  String? lastNameValue;
  String? lastEmailValue;

  @override
  Future<AccountState> build() async => _state;

  @override
  Future<void> updateName(String name) async {
    updateNameCalls++;
    lastNameValue = name;
  }

  @override
  Future<void> updateEmail(String email) async {
    updateEmailCalls++;
    lastEmailValue = email;
  }

  @override
  Future<void> changePassword({
    required String currentPassword,
    required String newPassword,
  }) async {
    changePasswordCalls++;
  }
}
