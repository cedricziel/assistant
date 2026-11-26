@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('MessageEditor text only ${brightness.name}', (tester) async {
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'Summarise the quarterly report in three bullets.',
          onSubmit: (_) {},
          onCancel: () {},
        ),
        size: const Size(400, 200),
        brightness: brightness,
      );

      await expectLater(
        find.byType(MessageEditor),
        matchesGoldenFile('goldens/message_editor_${brightness.name}.png'),
      );
    });

    testWidgets('MessageEditor with attachments ${brightness.name}', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'Summarise the attached files.',
          onSubmit: (_) {},
          onCancel: () {},
          attachments: const [
            Chip(label: Text('report.pdf')),
            Chip(label: Text('photo.png')),
          ],
        ),
        size: const Size(400, 240),
        brightness: brightness,
      );

      await expectLater(
        find.byType(MessageEditor),
        matchesGoldenFile(
          'goldens/message_editor_attachments_${brightness.name}.png',
        ),
      );
    });
  }
}
