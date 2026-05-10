// Pins the login form's focus traversal + autofill behavior:
//
// - The credential fields (email + password) live inside an [AutofillGroup]
//   so the OS recognises them as a credential pair (Keychain, 1Password,
//   browser autofill).
// - Email autofocuses on screen entry; pressing Enter advances focus to
//   password (via FocusNode + onFieldSubmitted), pressing Enter on
//   password submits.
// - Tab traversal moves through fields in source order — the explicit
//   focus nodes make the order deterministic on macOS desktop.
//
// Spec: see commit message of fix/login-tab-and-autofill.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/login/login_screen.dart';

GoRouter _router() => GoRouter(
  initialLocation: '/login',
  routes: [
    GoRoute(path: '/login', builder: (_, _) => const LoginScreen()),
    GoRoute(
      path: '/chat',
      builder: (_, _) => const Scaffold(body: Text('CHAT_PAGE')),
    ),
  ],
);

Future<void> _pump(WidgetTester tester) async {
  await tester.pumpWidget(
    ProviderScope(child: MaterialApp.router(routerConfig: _router())),
  );
  await tester.pumpAndSettle();
}

/// Find the rendered EditableText inside a TextFormField labelled [label].
/// TextFormField is a wrapper that ultimately produces a TextField; we
/// inspect the latter to assert focus, autofill, and autofocus behaviour.
EditableText _editable(WidgetTester tester, String label) {
  final field = find.ancestor(
    of: find.text(label),
    matching: find.byType(TextFormField),
  );
  expect(field, findsOneWidget, reason: 'TextFormField labelled "$label"');
  return tester.widget<EditableText>(
    find.descendant(of: field, matching: find.byType(EditableText)),
  );
}

void main() {
  testWidgets('email + password live inside an AutofillGroup', (tester) async {
    await _pump(tester);

    expect(
      find.byType(AutofillGroup),
      findsOneWidget,
      reason: 'Login form must be wrapped in an AutofillGroup',
    );
  });

  testWidgets('email field autofocuses on entry', (tester) async {
    await _pump(tester);

    final email = _editable(tester, 'Email');
    expect(
      email.autofocus,
      isTrue,
      reason: 'Email should autofocus so the user can start typing immediately',
    );
  });

  testWidgets('email autofill hints include username and email', (
    tester,
  ) async {
    await _pump(tester);

    final email = _editable(tester, 'Email');
    expect(email.autofillHints, contains(AutofillHints.username));
    expect(email.autofillHints, contains(AutofillHints.email));
  });

  testWidgets('password autofill hints include password', (tester) async {
    await _pump(tester);

    final password = _editable(tester, 'Password');
    expect(password.autofillHints, contains(AutofillHints.password));
  });

  testWidgets('Enter on email moves focus to password', (tester) async {
    await _pump(tester);

    // Type into email and trigger the "next" action (the keyboard's
    // Return/Tab equivalent) — focus should advance to password.
    await tester.enterText(
      find.ancestor(
        of: find.text('Email'),
        matching: find.byType(TextFormField),
      ),
      'admin@localhost',
    );
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pumpAndSettle();

    final password = _editable(tester, 'Password');
    expect(
      password.focusNode.hasFocus,
      isTrue,
      reason:
          'Pressing Next/Enter on the email field must move focus to password',
    );
  });
}
