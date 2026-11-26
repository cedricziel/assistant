@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

void main() {
  final cases = <String, bool>{
    'composer_voice_button_idle': false,
    'composer_voice_button_recording': true,
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Center(
            child: ComposerVoiceButton(
              isRecording: entry.value,
              elapsed: const Duration(seconds: 42),
              onStart: () {},
              onStop: () {},
            ),
          ),
          size: const Size(200, 64),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
