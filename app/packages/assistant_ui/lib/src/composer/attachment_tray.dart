import 'package:material_ui/material_ui.dart';

import '../models/pending_attachment.dart';
import '../theme/assistant_ui_spacing.dart';

bool _isImageMimeType(String mime) => mime.startsWith('image/');

IconData _iconForMime(String mime) {
  return switch (mime) {
    'application/pdf' => Icons.picture_as_pdf,
    'text/markdown' => Icons.description,
    'text/csv' => Icons.table_chart,
    'application/json' => Icons.data_object,
    'text/plain' => Icons.text_snippet,
    _ => Icons.insert_drive_file,
  };
}

/// A horizontal strip of attachments waiting to be sent: an image thumbnail
/// for images, an icon tile with the (shortened) file name for everything else.
///
/// Presentation only. The tray never mutates [attachments]; each remove
/// button emits [onRemove] with its index and the owner drops the entry.
/// Picking and dropping files stays with the app.
class AttachmentTray extends StatelessWidget {
  const AttachmentTray({
    super.key,
    required this.attachments,
    required this.onRemove,
  });

  final List<PendingAttachment> attachments;

  /// The user removed the attachment at this index.
  final void Function(int index) onRemove;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(
        AssistantUiSpacing.md,
        AssistantUiSpacing.sm,
        AssistantUiSpacing.md,
        0,
      ),
      child: SizedBox(
        height: 72,
        child: ListView.separated(
          scrollDirection: Axis.horizontal,
          itemCount: attachments.length,
          separatorBuilder: (_, _) =>
              const SizedBox(width: AssistantUiSpacing.sm),
          itemBuilder: (context, index) {
            final attachment = attachments[index];
            return Stack(
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: _isImageMimeType(attachment.mimeType)
                      ? Image.memory(
                          attachment.bytes,
                          width: 64,
                          height: 64,
                          fit: BoxFit.cover,
                        )
                      : Container(
                          width: 64,
                          height: 64,
                          color: colorScheme.surfaceContainerHighest,
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(
                                _iconForMime(attachment.mimeType),
                                size: 28,
                                color: colorScheme.onSurfaceVariant,
                              ),
                              const SizedBox(height: 2),
                              Text(
                                attachment.filename.length > 8
                                    ? '${attachment.filename.substring(0, 8)}…'
                                    : attachment.filename,
                                style: Theme.of(context).textTheme.labelSmall
                                    ?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                        ),
                ),
                Positioned(
                  top: -4,
                  right: -4,
                  child: IconButton(
                    key: Key('remove_attachment_$index'),
                    icon: const Icon(Icons.cancel, size: 18),
                    onPressed: () => onRemove(index),
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(),
                    color: colorScheme.error,
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
