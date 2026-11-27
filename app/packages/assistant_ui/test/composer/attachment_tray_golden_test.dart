@Tags(['golden'])
library;

import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('attachment_tray_files (${brightness.name})', (tester) async {
      await pumpGolden(
        tester,
        AttachmentTray(
          attachments: [
            PendingAttachment(
              bytes: Uint8List(8),
              filename: 'quarterly-report.pdf',
              mimeType: 'application/pdf',
            ),
            PendingAttachment(
              bytes: Uint8List(8),
              filename: 'data.csv',
              mimeType: 'text/csv',
            ),
            PendingAttachment(
              bytes: Uint8List(8),
              filename: 'notes.md',
              mimeType: 'text/markdown',
            ),
          ],
          onRemove: (_) {},
        ),
        size: const Size(300, 100),
        brightness: brightness,
      );
      await tester.pump(const Duration(milliseconds: 50));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/attachment_tray_files_${brightness.name}.png',
        ),
      );
    });
  }
}
