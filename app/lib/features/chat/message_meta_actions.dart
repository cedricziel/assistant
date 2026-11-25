import 'dart:async';

import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';
import '../../services/share_service.dart';
import '../../api/models/server_capabilities.dart';
import 'chat_provider.dart';
import 'read_aloud_action.dart';

/// Contextual action row rendered below each message bubble.
///
/// Shows actions like Copy, Read aloud, Retry, and Stop depending on
/// the message type and state.
class MetaActionRow extends StatelessWidget {
  const MetaActionRow({
    super.key,
    required this.message,
    required this.capabilities,
    required this.fetchMessageAudio,
    this.onRetry,
    this.onStop,
  });

  final ChatMessage message;
  final ServerCapabilities capabilities;
  final Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio;
  final VoidCallback? onRetry;
  final VoidCallback? onStop;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final isFailed = message.status == MessageStatus.failed;
    final colorScheme = Theme.of(context).colorScheme;
    final mutedColor = colorScheme.onSurface.withValues(alpha: 0.6);

    if (message.isStreaming) {
      if (onStop == null) return const SizedBox.shrink();
      return Padding(
        padding: const EdgeInsets.only(top: 4),
        child: MetaActionButton(
          key: const Key('stop_action'),
          icon: Icons.stop_rounded,
          label: 'Stop',
          color: mutedColor,
          onTap: onStop!,
        ),
      );
    }

    final actions = <Widget>[];

    // Read aloud: assistant messages without real audio, when TTS is available.
    if (!isUser &&
        capabilities.voiceReceive &&
        message.audioId == null &&
        message.content.isNotEmpty) {
      actions.add(
        ReadAloudAction(
          key: const Key('read_aloud_action'),
          fetchAudio: fetchMessageAudio,
          color: mutedColor,
        ),
      );
    }

    // Copy: all messages with content.
    if (message.content.isNotEmpty) {
      actions.add(
        MetaActionButton(
          key: const Key('copy_action'),
          icon: Icons.copy_outlined,
          label: 'Copy',
          color: mutedColor,
          onTap: () {
            Clipboard.setData(ClipboardData(text: message.content));
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Copied to clipboard'),
                duration: Duration(seconds: 1),
              ),
            );
          },
        ),
      );
    }

    // Share: assistant messages with content (text selection toolbar
    // handles sharing for user messages).
    if (!isUser && message.content.isNotEmpty) {
      actions.add(
        MetaActionButton(
          key: const Key('share_action'),
          icon: Icons.share_outlined,
          label: 'Share',
          color: mutedColor,
          onTap: () {
            final box = context.findRenderObject() as RenderBox?;
            final origin = box != null
                ? box.localToGlobal(Offset.zero) & box.size
                : null;
            ShareService.instance.shareText(
              message.content,
              sharePositionOrigin: origin,
            );
          },
        ),
      );
    }

    // Save: assistant messages with content.
    if (!isUser && message.content.isNotEmpty) {
      actions.add(
        MetaActionButton(
          key: const Key('save_action'),
          icon: Icons.save_alt_outlined,
          label: 'Save',
          color: mutedColor,
          onTap: () {
            final box = context.findRenderObject() as RenderBox?;
            final origin = box != null
                ? box.localToGlobal(Offset.zero) & box.size
                : null;
            ShareService.instance.saveText(
              message.content,
              suggestedFilename:
                  'assistant-message-${DateTime.now().toIso8601String().split('T').first}.md',
              sharePositionOrigin: origin,
            );
          },
        ),
      );
    }

    // Retry: failed messages only.
    if (isFailed && onRetry != null) {
      actions.add(
        MetaActionButton(
          key: const Key('retry_button'),
          icon: Icons.refresh,
          label: 'Retry',
          color: colorScheme.error,
          onTap: onRetry!,
        ),
      );
    }

    if (actions.isEmpty) return const SizedBox.shrink();

    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Wrap(
        alignment: isUser ? WrapAlignment.end : WrapAlignment.start,
        spacing: 16,
        runSpacing: 4,
        children: actions,
      ),
    );
  }
}

/// A small icon + label button used in the meta action row.
class MetaActionButton extends StatelessWidget {
  const MetaActionButton({
    super.key,
    required this.icon,
    required this.label,
    required this.onTap,
    required this.color,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 14, color: color),
            const SizedBox(width: 4),
            Text(label, style: TextStyle(fontSize: 12, color: color)),
          ],
        ),
      ),
    );
  }
}
