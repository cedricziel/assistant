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
/// Share, Save, Retry; [isStopping] keeps Stop visible but disabled) and feeds
/// them to [MessageActions]. The response timing indicator (only for finished
/// replies that have a timing, and only while [showResponseTiming]) and
/// [feedback] fill the trailing slot, in that order.
///
/// Branching: Edit ([onEdit]) is offered for persisted user messages and
/// Regenerate ([onRegenerate]) for persisted assistant messages, only while no
/// turn works ([turnWorking] is false while idle and while a turn only waits
/// for the user). [onRegenerateWith] adds a model menu when [regenerateModels]
/// has two or more entries. A message with `branch` gets a [BranchPicker] in
/// the leading slot, wired to [onSwitchBranch] with the id of the sibling
/// version to show; the picker is busy while [turnWorking]. All of these stay off until their callbacks are
/// supplied.
Widget buildMessageActions({
  required ChatMessage message,
  required ServerCapabilities capabilities,
  required Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio,
  VoidCallback? onRetry,
  VoidCallback? onStop,
  bool isStopping = false,
  bool showResponseTiming = false,
  Widget? feedback,
  bool isPersisted = false,
  bool turnWorking = false,
  VoidCallback? onEdit,
  VoidCallback? onRegenerate,
  ValueChanged<String>? onRegenerateWith,
  List<RegenerateModel> regenerateModels = const [],
  ValueChanged<String>? onSwitchBranch,
}) {
  final isUser = message.isUser;
  final alignment = isUser ? WrapAlignment.end : WrapAlignment.start;
  final timing = !isUser && !message.isStreaming && showResponseTiming
      ? message.timing
      : null;

  if (message.isStreaming) {
    if (onStop == null) return const SizedBox.shrink();
    return MessageActions(
      actions: [
        MessageActionButton(
          key: const Key('stop_action'),
          icon: Icons.stop_rounded,
          label: isStopping ? 'Stopping…' : 'Stop',
          onPressed: isStopping ? null : onStop,
        ),
      ],
    );
  }

  final hasContent = message.content.isNotEmpty;
  final isFailed = message.status == MessageStatus.failed;
  final branch = message.branch;
  final canBranch = isPersisted && !turnWorking;
  final showEdit = canBranch && isUser && onEdit != null;
  final showRegenerate = canBranch && !isUser && onRegenerate != null;
  final showRegenerateMenu =
      canBranch &&
      !isUser &&
      onRegenerateWith != null &&
      regenerateModels.length >= 2;

  return Builder(
    builder: (context) {
      final mutedColor = Theme.of(context).colorScheme.onSurface
          .withValues(alpha: 0.6);

      return MessageActions(
        alignment: alignment,
        leading: branch != null && onSwitchBranch != null
            ? BranchPicker(
                index: branch.index,
                count: branch.count,
                busy: turnWorking,
                onPrevious: () =>
                    onSwitchBranch(branch.siblingIds[branch.index - 1]),
                onNext: () =>
                    onSwitchBranch(branch.siblingIds[branch.index + 1]),
              )
            : null,
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
          // Edit and Regenerate: persisted messages, while no turn works.
          if (showEdit)
            MessageActionButton(
              key: const Key('edit_action'),
              icon: Icons.edit_outlined,
              label: 'Edit',
              onPressed: onEdit,
            ),
          if (showRegenerate)
            MessageActionButton(
              key: const Key('regenerate_action'),
              icon: Icons.refresh,
              label: 'Regenerate',
              onPressed: onRegenerate,
            ),
          if (showRegenerateMenu)
            RegenerateMenuButton(
              key: const Key('regenerate_with_action'),
              models: regenerateModels,
              onSelected: onRegenerateWith,
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
        trailing: [
          if (timing != null) MessageTimingIndicator(timing: timing),
          ?feedback,
        ],
      );
    },
  );
}

Rect? _origin(BuildContext context) {
  final box = context.findRenderObject() as RenderBox?;
  return box != null ? box.localToGlobal(Offset.zero) & box.size : null;
}
