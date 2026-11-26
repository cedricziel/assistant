import 'package:flutter/services.dart';
import 'package:assistant_ui/assistant_ui.dart';

import '../../api/models/server_capabilities.dart';
import '../../services/share_service.dart';
import '../../shared/platform/widgets.dart';
import 'chat_provider.dart';
import 'read_aloud_action.dart';

/// Builds the action row shown below a message bubble.
///
/// Decides which actions apply to [message] (Stop, Stopped, Read aloud, Copy,
/// Share, Save, Retry) and feeds them to [MessageActions]. [timing] and
/// [feedback] fill the trailing slot, in that order, once the features that
/// supply them exist.
Widget buildMessageActions({
  required ChatMessage message,
  required ServerCapabilities capabilities,
  required Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio,
  VoidCallback? onRetry,
  VoidCallback? onStop,
  Widget? timing,
  Widget? feedback,
}) {
  final isUser = message.isUser;
  final alignment = isUser ? WrapAlignment.end : WrapAlignment.start;

  if (message.isStreaming) {
    if (onStop == null) return const SizedBox.shrink();
    return MessageActions(
      actions: [
        MessageActionButton(
          key: const Key('stop_action'),
          icon: Icons.stop_rounded,
          label: 'Stop',
          onPressed: onStop,
        ),
      ],
    );
  }

  final hasContent = message.content.isNotEmpty;
  final isFailed = message.status == MessageStatus.failed;

  return Builder(
    builder: (context) {
      final mutedColor = Theme.of(context).colorScheme.onSurface
          .withValues(alpha: 0.6);

      return MessageActions(
        alignment: alignment,
        actions: [
          // Stopped: the reply was cut short by Stop.
          if (!isUser && message.status == MessageStatus.stopped)
            Row(
              key: const Key('stopped_label'),
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.stop_circle_outlined, size: 14, color: mutedColor),
                const SizedBox(width: 4),
                Text(
                  'Stopped',
                  style: TextStyle(fontSize: 12, color: mutedColor),
                ),
              ],
            ),
          // Read aloud: assistant messages without real audio, when TTS is
          // available.
          if (!isUser &&
              capabilities.voiceReceive &&
              message.audioId == null &&
              hasContent)
            ReadAloudAction(
              key: const Key('read_aloud_action'),
              fetchAudio: fetchMessageAudio,
            ),
          // Copy: all messages with content.
          if (hasContent)
            MessageActionButton(
              key: const Key('copy_action'),
              icon: Icons.copy_outlined,
              label: 'Copy',
              onPressed: () {
                Clipboard.setData(ClipboardData(text: message.content));
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Copied to clipboard'),
                    duration: Duration(seconds: 1),
                  ),
                );
              },
            ),
          // Share and Save: assistant messages with content (the text
          // selection toolbar handles sharing for user messages).
          if (!isUser && hasContent)
            MessageActionButton(
              key: const Key('share_action'),
              icon: Icons.share_outlined,
              label: 'Share',
              onPressed: () => ShareService.instance.shareText(
                message.content,
                sharePositionOrigin: _origin(context),
              ),
            ),
          if (!isUser && hasContent)
            MessageActionButton(
              key: const Key('save_action'),
              icon: Icons.save_alt_outlined,
              label: 'Save',
              onPressed: () => ShareService.instance.saveText(
                message.content,
                suggestedFilename:
                    'assistant-message-${DateTime.now().toIso8601String().split('T').first}.md',
                sharePositionOrigin: _origin(context),
              ),
            ),
          // Retry: failed messages only.
          if (isFailed && onRetry != null)
            MessageActionButton(
              key: const Key('retry_button'),
              icon: Icons.refresh,
              label: 'Retry',
              tone: MessageActionTone.danger,
              onPressed: onRetry,
            ),
        ],
        trailing: [?timing, ?feedback],
      );
    },
  );
}

Rect? _origin(BuildContext context) {
  final box = context.findRenderObject() as RenderBox?;
  return box != null ? box.localToGlobal(Offset.zero) & box.size : null;
}
