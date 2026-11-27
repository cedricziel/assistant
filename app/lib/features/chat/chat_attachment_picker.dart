import 'package:desktop_drop/desktop_drop.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_ui/models.dart';

import '../../api/connectivity_provider.dart';
import 'attachment_provider.dart';
import 'image_utils.dart';

/// Opens the app's file picker and queues the chosen files as attachments.
///
/// Attachments need the server, so this refuses to pick while offline. Image
/// files are normalized (HEIC to PNG) before they reach the composer tray.
Future<void> pickChatAttachments(WidgetRef ref, BuildContext context) async {
  if (!ref.read(isOnlineProvider)) {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('File upload requires an internet connection'),
      ),
    );
    return;
  }

  final files = await FilePicker.pickFiles(
    type: FileType.custom,
    allowedExtensions: const [
      'png',
      'jpg',
      'jpeg',
      'gif',
      'webp',
      'heic',
      'heif',
      'pdf',
      'txt',
      'md',
      'csv',
      'json',
    ],
  );
  final notifier = ref.read(pendingAttachmentsProvider.notifier);
  for (final file in files) {
    final bytes = await file.readAsBytes();
    final mime = mimeFromExtension(file.extension);
    if (isImageMimeType(mime) || mime == 'image/heic') {
      final (normalized, filename, normalizedMime) = await normalizeImage(
        bytes,
        file.name,
        file.extension,
      );
      notifier.add(
        PendingAttachment(
          bytes: normalized,
          filename: filename,
          mimeType: normalizedMime,
        ),
      );
    } else {
      notifier.add(
        PendingAttachment(bytes: bytes, filename: file.name, mimeType: mime),
      );
    }
  }
}

/// Queues files dropped onto the chat area, skipping unsupported types.
void addDroppedAttachments(WidgetRef ref, List<DropItem> files) {
  final notifier = ref.read(pendingAttachmentsProvider.notifier);
  for (final file in files) {
    file.readAsBytes().then((bytes) {
      final extension = file.name.split('.').last;
      final mime = mimeFromExtension(extension);
      if (isSupportedMimeType(mime)) {
        notifier.add(
          PendingAttachment(bytes: bytes, filename: file.name, mimeType: mime),
        );
      }
    });
  }
}
