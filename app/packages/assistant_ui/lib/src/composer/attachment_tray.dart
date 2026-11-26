import 'package:material_ui/material_ui.dart';

import '../models/pending_attachment.dart';
import '../theme/assistant_ui_spacing.dart';

/// Edge of an attachment tile.
const double _kTileSize = 64;

/// Hit box of the remove control (WCAG 2.2 SC 2.5.8 minimum).
const double _kRemoveSize = 24;

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
                          width: _kTileSize,
                          height: _kTileSize,
                          fit: BoxFit.cover,
                        )
                      : Container(
                          width: _kTileSize,
                          height: _kTileSize,
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
                // Inside the tile's top-right corner, so every point of the
                // 24 dp hit box receives taps (a part outside the tile would
                // be clipped by the list).
                Positioned(
                  top: 0,
                  right: 0,
                  child: IconButton(
                    key: Key('remove_attachment_$index'),
                    icon: const Icon(Icons.cancel, size: 18),
                    tooltip: 'Remove ${attachment.filename}',
                    onPressed: () => onRemove(index),
                    color: colorScheme.error,
                    // Without shrinkWrap Material pads the hit box to 48 dp,
                    // which would reach outside the tile.
                    style: IconButton.styleFrom(
                      padding: EdgeInsets.zero,
                      minimumSize: const Size.square(_kRemoveSize),
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    ),
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
