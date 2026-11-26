@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _full = MessageTiming(
  timeToFirstToken: Duration(milliseconds: 800),
  total: Duration(milliseconds: 12400),
  outputTokens: 420,
  decode: Duration(seconds: 10),
);
const _totalOnly = MessageTiming(total: Duration(milliseconds: 65000));

void main() {
  // name -> (timing, whether keyboard focus is moved onto the indicator).
  final cases = <String, ({MessageTiming timing, bool focused})>{
    'message_timing_indicator_full': (timing: _full, focused: false),
    'message_timing_indicator_total_only': (timing: _totalOnly, focused: false),
    'message_timing_indicator_focused': (timing: _full, focused: true),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Padding(
            padding: const EdgeInsets.all(16),
            child: MessageTimingIndicator(timing: entry.value.timing),
          ),
          size: const Size(400, 200),
          brightness: brightness,
        );
        if (entry.value.focused) {
          await tester.sendKeyEvent(LogicalKeyboardKey.tab);
          // The tooltip is inserted a frame after the focus change.
          await tester.pump();
        }
        await tester.pump(const Duration(milliseconds: 500));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
