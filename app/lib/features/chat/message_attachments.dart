import 'dart:async';

import 'package:http/http.dart' as http;
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';
import '../../services/share_service.dart';
import 'chat_provider.dart';
import 'image_utils.dart';

/// Thumbnail grid for the attachments on a chat message.
///
/// Images load through the authenticated media endpoint and open full-size on
/// tap; other files render as a file tile. Long-press menus offer save, share
/// and copy.
class MessageAttachments extends StatelessWidget {
  const MessageAttachments({
    super.key,
    required this.attachments,
    required this.isUser,
    this.imageBaseUrl,
    this.imageAuthToken,
  });

  final List<ChatAttachment> attachments;
  final bool isUser;
  final String? imageBaseUrl;
  final String? imageAuthToken;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: EdgeInsets.only(bottom: isUser ? 6 : 0, top: isUser ? 0 : 6),
      child: Wrap(
        spacing: 6,
        runSpacing: 6,
        children: attachments.map((att) {
          // Non-image attachments: render as file tile with icon.
          if (!isImageMimeType(att.mimeType)) {
            return Container(
              width: 150,
              height: 150,
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    iconForMime(att.mimeType),
                    color: colorScheme.outline,
                    size: 32,
                  ),
                  const SizedBox(height: 6),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    child: Text(
                      att.filename,
                      style: TextStyle(
                        fontSize: 10,
                        color: colorScheme.outline,
                      ),
                      overflow: TextOverflow.ellipsis,
                      textAlign: TextAlign.center,
                    ),
                  ),
                ],
              ),
            );
          }

          // Without a base URL the relative path cannot be resolved.
          if (imageBaseUrl == null) {
            return Container(
              width: 150,
              height: 150,
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.image, color: colorScheme.outline),
                  const SizedBox(height: 4),
                  Text(
                    att.filename,
                    style: TextStyle(fontSize: 10, color: colorScheme.outline),
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            );
          }
          final thumbUrl = '$imageBaseUrl${att.url}?w=300';
          return AdaptiveMediaContextMenu(
            actions: _imageContextMenuActions(context, att),
            child: GestureDetector(
              onTap: () => _showFullImage(context, att),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: CachedNetworkImage(
                  imageUrl: thumbUrl,
                  width: 150,
                  height: 150,
                  fit: BoxFit.cover,
                  httpHeaders: imageAuthToken != null
                      ? {'Authorization': 'Bearer $imageAuthToken'}
                      : const {},
                  placeholder: (_, _) => Container(
                    width: 150,
                    height: 150,
                    color: colorScheme.surfaceContainerLowest,
                    child: const Center(
                      child: SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator.adaptive(
                          strokeWidth: 2,
                        ),
                      ),
                    ),
                  ),
                  errorWidget: (_, url, _) => GestureDetector(
                    onTap: () {
                      CachedNetworkImage.evictFromCache(url);
                      // Trigger a rebuild to retry loading.
                      (context as Element).markNeedsBuild();
                    },
                    child: Container(
                      width: 150,
                      height: 150,
                      color: colorScheme.surfaceContainerLowest,
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(Icons.refresh, color: colorScheme.outline),
                          const SizedBox(height: 4),
                          Text(
                            'Tap to retry',
                            style: TextStyle(
                              fontSize: 10,
                              color: colorScheme.outline,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
        }).toList(),
      ),
    );
  }

  List<AdaptiveContextMenuAction> _imageContextMenuActions(
    BuildContext context,
    ChatAttachment attachment,
  ) {
    return [
      AdaptiveContextMenuAction(
        label: 'Save Image',
        cupertinoIcon: CupertinoIcons.square_arrow_down,
        materialIcon: Icons.save_alt,
        onPressed: () => _saveAttachment(context, attachment),
      ),
      AdaptiveContextMenuAction(
        label: 'Share',
        cupertinoIcon: CupertinoIcons.share,
        materialIcon: Icons.share,
        onPressed: () => _shareAttachment(context, attachment),
      ),
      AdaptiveContextMenuAction(
        label: 'Copy Image',
        cupertinoIcon: CupertinoIcons.doc_on_clipboard,
        materialIcon: Icons.copy,
        onPressed: () => _copyAttachmentToClipboard(context, attachment),
      ),
    ];
  }

  Future<Uint8List?> _fetchAttachmentBytes(ChatAttachment attachment) async {
    if (imageBaseUrl == null) return null;
    final url = '$imageBaseUrl${attachment.url}';
    try {
      final headers = <String, String>{};
      if (imageAuthToken != null) {
        headers['Authorization'] = 'Bearer $imageAuthToken';
      }
      final response = await http.get(Uri.parse(url), headers: headers);
      if (response.statusCode == 200) {
        return response.bodyBytes;
      }
      return null;
    } catch (_) {
      return null;
    }
  }

  Future<void> _saveAttachment(
    BuildContext context,
    ChatAttachment attachment,
  ) async {
    final box = context.findRenderObject() as RenderBox?;
    final origin = box != null
        ? box.localToGlobal(Offset.zero) & box.size
        : null;
    final bytes = await _fetchAttachmentBytes(attachment);
    if (bytes == null) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Failed to download image')),
        );
      }
      return;
    }
    await ShareService.instance.saveFile(
      bytes,
      filename: attachment.filename,
      mimeType: attachment.mimeType,
      sharePositionOrigin: origin,
    );
  }

  Future<void> _shareAttachment(
    BuildContext context,
    ChatAttachment attachment,
  ) async {
    final box = context.findRenderObject() as RenderBox?;
    final origin = box != null
        ? box.localToGlobal(Offset.zero) & box.size
        : null;
    final bytes = await _fetchAttachmentBytes(attachment);
    if (bytes == null) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Failed to download image')),
        );
      }
      return;
    }
    await ShareService.instance.shareFile(
      bytes,
      filename: attachment.filename,
      mimeType: attachment.mimeType,
      sharePositionOrigin: origin,
    );
  }

  Future<void> _copyAttachmentToClipboard(
    BuildContext context,
    ChatAttachment attachment,
  ) async {
    final bytes = await _fetchAttachmentBytes(attachment);
    if (bytes == null) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Failed to download image')),
        );
      }
      return;
    }
    await Clipboard.setData(ClipboardData(text: attachment.filename));
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Image copied'),
          duration: Duration(seconds: 1),
        ),
      );
    }
  }

  void _showFullImage(BuildContext context, ChatAttachment attachment) {
    if (imageBaseUrl == null) return;
    final fullUrl = '$imageBaseUrl${attachment.url}?w=1920';
    showDialog(
      context: context,
      builder: (ctx) => Dialog(
        backgroundColor: Colors.transparent,
        insetPadding: const EdgeInsets.all(16),
        child: Stack(
          alignment: Alignment.topRight,
          children: [
            InteractiveViewer(
              child: CachedNetworkImage(
                imageUrl: fullUrl,
                fit: BoxFit.contain,
                httpHeaders: imageAuthToken != null
                    ? {'Authorization': 'Bearer $imageAuthToken'}
                    : const {},
                placeholder: (_, _) =>
                    const Center(child: CircularProgressIndicator.adaptive()),
                errorWidget: (_, _, _) => Center(
                  child: Icon(
                    Icons.broken_image,
                    size: 64,
                    color: Theme.of(context).colorScheme.onSurface
                        .withValues(alpha: 0.7),
                  ),
                ),
              ),
            ),
            IconButton(
              icon: Icon(
                Icons.close,
                color: Theme.of(context).colorScheme.onPrimary,
              ),
              onPressed: () => Navigator.of(ctx).pop(),
            ),
          ],
        ),
      ),
    );
  }
}
