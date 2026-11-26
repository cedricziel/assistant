import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/shared/platform/adaptive_dialog.dart';

void _setPlatform(TargetPlatform platform) {
  debugDefaultTargetPlatformOverride = platform;
  addTearDown(() => debugDefaultTargetPlatformOverride = null);
}

/// Pumps a button that opens the dialog and stores its result in [result].
Future<void> _open(
  WidgetTester tester, {
  required List<String?> result,
  String initialText = 'Old title',
  int maxLength = 200,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (ctx) => TextButton(
            onPressed: () async {
              final value = await showAdaptiveTextInputDialog(
                context: ctx,
                title: 'Rename',
                initialText: initialText,
                confirmLabel: 'Save',
                maxLength: maxLength,
              );
              result.add(value);
            },
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
}

Finder get _saveButton => find.ancestor(
  of: find.text('Save'),
  matching: find.byWidgetPredicate(
    (w) => w is TextButton || w is CupertinoDialogAction,
  ),
);

bool _saveEnabled(WidgetTester tester) {
  final w = tester.widget(_saveButton);
  if (w is TextButton) return w.onPressed != null;
  return (w as CupertinoDialogAction).onPressed != null;
}

void main() {
  group('showAdaptiveTextInputDialog', () {
    testWidgets('renders CupertinoAlertDialog on iOS', (tester) async {
      _setPlatform(TargetPlatform.iOS);
      await _open(tester, result: []);

      expect(find.byType(CupertinoAlertDialog), findsOneWidget);
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(CupertinoTextField), findsOneWidget);
    });

    for (final platform in [TargetPlatform.macOS, TargetPlatform.android]) {
      testWidgets('renders AlertDialog on $platform', (tester) async {
        _setPlatform(platform);
        await _open(tester, result: []);

        expect(find.byType(AlertDialog), findsOneWidget);
        expect(find.byType(CupertinoAlertDialog), findsNothing);
        expect(find.byType(TextField), findsOneWidget);
      });
    }

    testWidgets('is prefilled with the text selected', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      await _open(tester, result: []);

      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.controller!.text, 'Old title');
      expect(
        field.controller!.selection,
        const TextSelection(baseOffset: 0, extentOffset: 9),
      );
    });

    testWidgets('keeps Confirm disabled while blank', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      await _open(tester, result: []);
      expect(_saveEnabled(tester), isTrue);

      await tester.enterText(find.byType(TextField), '   ');
      await tester.pump();
      expect(_saveEnabled(tester), isFalse);

      await tester.enterText(find.byType(TextField), 'x');
      await tester.pump();
      expect(_saveEnabled(tester), isTrue);
    });

    testWidgets('counts maxLength in runes, not UTF-16 units', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      await _open(tester, result: []);

      // U+1F600 is one rune but two UTF-16 code units.
      await tester.enterText(find.byType(TextField), '😀' * 200);
      await tester.pump();
      expect(_saveEnabled(tester), isTrue, reason: '200 runes is allowed');

      await tester.enterText(find.byType(TextField), '😀' * 201);
      await tester.pump();
      expect(_saveEnabled(tester), isFalse, reason: '201 runes is too long');
    });

    testWidgets('confirms on Enter with the entered text', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.enterText(find.byType(TextField), 'New title');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      expect(result, ['New title']);
      expect(find.byType(AlertDialog), findsNothing);
    });

    testWidgets('Enter does not confirm while invalid', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.enterText(find.byType(TextField), '  ');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      expect(result, isEmpty);
      expect(find.byType(AlertDialog), findsOneWidget);
    });

    testWidgets('trims the returned text', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.enterText(find.byType(TextField), '  Padded  ');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(result, ['Padded']);
    });

    testWidgets('returns null on Cancel', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(result, [null]);
    });

    testWidgets('returns null on Esc', (tester) async {
      _setPlatform(TargetPlatform.macOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();

      expect(result, [null]);
      expect(find.byType(AlertDialog), findsNothing);
    });

    testWidgets('returns null on Cancel on iOS', (tester) async {
      _setPlatform(TargetPlatform.iOS);
      final result = <String?>[];
      await _open(tester, result: result);

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(result, [null]);
    });
  });
}
