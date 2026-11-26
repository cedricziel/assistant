import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _timing = MessageTiming(
  timeToFirstToken: Duration(milliseconds: 800),
  total: Duration(milliseconds: 12400),
  outputTokens: 420,
  decode: Duration(seconds: 10),
);
const _tooltipText = 'First token 0.8 s · Total 12.4 s · 42 tok/s';

Future<void> _pump(WidgetTester tester, {Widget? before}) {
  return tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: FocusTraversalGroup(
          child: Row(
            children: [
              ?before,
              const MessageTimingIndicator(timing: _timing),
            ],
          ),
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('shows nothing until it is used', (tester) async {
    await _pump(tester);

    expect(find.text(_tooltipText), findsNothing);
  });

  testWidgets('a tap shows the tooltip text', (tester) async {
    await _pump(tester);

    await tester.tap(find.byType(MessageTimingIndicator));
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text(_tooltipText), findsOneWidget);
  });

  testWidgets('a mouse hover shows the tooltip text', (tester) async {
    await _pump(tester);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    addTearDown(mouse.removePointer);

    await mouse.moveTo(tester.getCenter(find.byType(MessageTimingIndicator)));
    await tester.pump(const Duration(seconds: 1));

    expect(find.text(_tooltipText), findsOneWidget);
  });

  testWidgets('Tab from a preceding control shows it with no pointer event', (
    tester,
  ) async {
    await _pump(
      tester,
      before: const TextButton(onPressed: _noop, child: Text('before')),
    );
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    expect(find.text(_tooltipText), findsNothing, reason: 'on the button');

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text(_tooltipText), findsOneWidget);
  });

  testWidgets('Enter on the focused indicator shows it', (tester) async {
    await _pump(tester);
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump(const Duration(milliseconds: 100));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump(const Duration(seconds: 1));
    expect(find.text(_tooltipText), findsNothing, reason: 'dismissed');

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text(_tooltipText), findsOneWidget);
  });

  testWidgets('is a focusable button labelled with the spoken sentence', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await _pump(tester);

    expect(
      tester.getSemantics(find.bySemanticsLabel(RegExp('^Response timing:'))),
      matchesSemantics(
        label:
            'Response timing: first token 0.8 seconds, total 12.4 seconds, '
            '42 tokens per second',
        isButton: true,
        isFocusable: true,
        hasTapAction: true,
        hasFocusAction: true,
      ),
    );
    handle.dispose();
  });

  testWidgets('the hit area is at least 24 by 24', (tester) async {
    await _pump(tester);

    final size = tester.getSize(find.byType(MessageTimingIndicator));

    expect(size.width, greaterThanOrEqualTo(24));
    expect(size.height, greaterThanOrEqualTo(24));
  });

  testWidgets('draws the speed glyph', (tester) async {
    await _pump(tester);

    expect(find.byIcon(Icons.speed_outlined), findsOneWidget);
  });
}

void _noop() {}
