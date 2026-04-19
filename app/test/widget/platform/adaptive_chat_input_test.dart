import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

Widget _harness(Widget screen) {
  return ProviderScope(
    overrides: [
      apiClientProvider.overrideWithValue(null),
      pwaInstallProvider.overrideWith(_FakePwaNotifier.new),
    ],
    child: MaterialApp(home: Material(child: screen)),
  );
}

void main() {
  // 4.3.1 — Chat input uses CupertinoTextField on iOS
  group('Chat input — adaptive text field', () {
    testWidgets('CupertinoTextField on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const ChatScreen()));
      await tester.pump();

      expect(
        find.byType(CupertinoTextField),
        findsOneWidget,
        reason: 'Chat input should use CupertinoTextField on iOS',
      );
      expect(
        find.byType(TextField),
        findsNothing,
        reason: 'Chat input should NOT use Material TextField on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('Material TextField on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const ChatScreen()));
      await tester.pump();

      expect(
        find.byType(TextField),
        findsOneWidget,
        reason: 'Chat input should use Material TextField on macOS',
      );
      expect(
        find.byType(CupertinoTextField),
        findsNothing,
        reason: 'Chat input should NOT use CupertinoTextField on macOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    // 4.3.3 — Functional parity: enter text, submit
    testWidgets('CupertinoTextField submit triggers onSend', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const ChatScreen()));
      await tester.pump();

      final inputFinder = find.byType(CupertinoTextField);
      expect(inputFinder, findsOneWidget);

      // Enter text.
      await tester.enterText(inputFinder, 'Hello');
      // Advance past the 350ms keyboard-scroll timer.
      await tester.pump(const Duration(milliseconds: 400));

      // Verify the text was entered.
      final textField = tester.widget<CupertinoTextField>(inputFinder);
      expect(textField.controller?.text, 'Hello');

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
