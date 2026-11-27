import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:widgetbook/widgetbook.dart';

final attachmentTrayComponent = WidgetbookComponent(
  name: 'AttachmentTray',
  useCases: [
    WidgetbookUseCase(
      name: 'files',
      builder: (context) => AttachmentTray(
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
    ),
  ],
);
