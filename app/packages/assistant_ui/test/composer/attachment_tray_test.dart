import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

// A 1x1 transparent PNG, so `Image.memory` decodes without errors.
final Uint8List _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA'
  '60e6kgAAAABJRU5ErkJggg==',
);

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  final attachments = [
    PendingAttachment(
      bytes: _png,
      filename: 'photo.png',
      mimeType: 'image/png',
    ),
    PendingAttachment(
      bytes: Uint8List(8),
      filename: 'quarterly-report.pdf',
      mimeType: 'application/pdf',
    ),
  ];

  testWidgets(
    'remove button fires onRemove with its index and leaves the list',
    (tester) async {
      final removed = <int>[];
      await tester.pumpWidget(
        _host(AttachmentTray(attachments: attachments, onRemove: removed.add)),
      );

      await tester.tap(find.byKey(const Key('remove_attachment_0')));
      await tester.pump();

      expect(removed, [0], reason: 'onRemove receives the tapped index');
      expect(
        find.byKey(const Key('remove_attachment_1')),
        findsOneWidget,
        reason: 'the tray is stateless: removal is the owner\'s job',
      );
      expect(attachments, hasLength(2), reason: 'the list is not mutated');
    },
  );

  testWidgets('renders an image thumbnail for images and a tile for files', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(AttachmentTray(attachments: attachments, onRemove: (_) {})),
    );

    expect(find.byType(Image), findsOneWidget);
    expect(find.byIcon(Icons.picture_as_pdf), findsOneWidget);
    expect(
      find.text('quarterl…'),
      findsOneWidget,
      reason: 'long file names are cut to eight characters',
    );
  });

  testWidgets('short file names are not truncated', (tester) async {
    await tester.pumpWidget(
      _host(
        AttachmentTray(
          attachments: [
            PendingAttachment(
              bytes: Uint8List(8),
              filename: 'a.csv',
              mimeType: 'text/csv',
            ),
          ],
          onRemove: (_) {},
        ),
      ),
    );

    expect(find.text('a.csv'), findsOneWidget);
    expect(find.byIcon(Icons.table_chart), findsOneWidget);
  });
}
