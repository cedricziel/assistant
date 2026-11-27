import 'dart:async';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/services.dart';

import '../../api/models/server_capabilities.dart';
import '../../services/share_service.dart';
import '../../shared/platform/widgets.dart';
import 'audio_player_widget.dart';
import 'chat_provider.dart';
import 'message_actions_builder.dart';
import 'message_attachments.dart';
import 'message_audio_actions.dart';
import 'voice_message_player.dart';

/// Feeds a [MessageBubble] the app's I/O: attachment thumbnails, the voice and
/// audio players inside platform context menus, the adaptive text selection
/// toolbar with Share and Save, and the action row below the bubble.
///
/// Markdown plugins, links and images reach the bubble through the thread's
/// `ChatMarkdownConfig`.
class ChatMessageBubbleAdapter extends StatelessWidget {
  const ChatMessageBubbleAdapter({
    super.key,
    required this.message,
    required this.senderLabel,
    required this.capabilities,
    required this.fetchMessageAudio,
    this.isGrouped = false,
    this.onRetry,
    this.onStop,
    this.isStopping = false,
    this.imageBaseUrl,
    this.imageAuthToken,
  });

  final ChatMessage message;

  /// Who the bubble speaks for: "You" for the user's own messages and the
  /// active Persona's name for assistant messages.
  final String senderLabel;
  final bool isGrouped;
  final VoidCallback? onRetry;
  final VoidCallback? onStop;
  final bool isStopping;
  final ServerCapabilities capabilities;
  final Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio;
  final String? imageBaseUrl;
  final String? imageAuthToken;

  @override
  Widget build(BuildContext context) {
    return MessageBubble(
      message: message,
      senderLabel: senderLabel,
      isGrouped: isGrouped,
      attachments: message.attachments.isEmpty
          ? null
          : MessageAttachments(
              attachments: message.attachments,
              isUser: message.isUser,
              imageBaseUrl: imageBaseUrl,
              imageAuthToken: imageAuthToken,
            ),
      audio: _audio(context),
      actions: buildMessageActions(
        message: message,
        capabilities: capabilities,
        fetchMessageAudio: fetchMessageAudio,
        onRetry: onRetry,
        onStop: onStop,
        isStopping: isStopping,
      ),
      contextMenuBuilder: _textSelectionContextMenuBuilder,
    );
  }

  /// A user's voice message, or the audio the agent produced for an assistant
  /// message once it has finished streaming.
  Widget? _audio(BuildContext context) {
    final audioBytes = message.audioBytes;
    if (message.isUser) {
      if (audioBytes == null) return null;
      return AdaptiveMediaContextMenu(
        actions: voiceMessageActions(
          context,
          audioBytes,
          message.audioMimeType ?? 'audio/webm',
        ),
        child: VoiceMessagePlayer(
          audioBytes: audioBytes,
          audioMimeType: message.audioMimeType,
          transcript: message.content,
          foregroundColor: Theme.of(context).colorScheme.onPrimary,
        ),
      );
    }
    if (message.audioId == null || message.isStreaming) return null;
    return AdaptiveMediaContextMenu(
      actions: ttsAudioActions(context, fetchMessageAudio),
      child: Padding(
        padding: const EdgeInsets.only(top: 6),
        child: AudioPlayerWidget(fetchAudio: fetchMessageAudio),
      ),
    );
  }

  Widget _textSelectionContextMenuBuilder(
    BuildContext context,
    EditableTextState editableTextState,
  ) {
    final selection = editableTextState.textEditingValue.selection;
    final selectedText = selection.isValid && !selection.isCollapsed
        ? editableTextState.textEditingValue.text.substring(
            selection.start,
            selection.end,
          )
        : message.content;

    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: editableTextState.contextMenuAnchors,
      buttonItems: [
        ...editableTextState.contextMenuButtonItems,
        ContextMenuButtonItem(
          label: 'Share',
          onPressed: () {
            editableTextState.hideToolbar();
            final box = context.findRenderObject() as RenderBox?;
            final origin = box != null
                ? box.localToGlobal(Offset.zero) & box.size
                : null;
            ShareService.instance.shareText(
              selectedText,
              sharePositionOrigin: origin,
            );
          },
        ),
        ContextMenuButtonItem(
          label: 'Save As…',
          onPressed: () {
            editableTextState.hideToolbar();
            final box = context.findRenderObject() as RenderBox?;
            final origin = box != null
                ? box.localToGlobal(Offset.zero) & box.size
                : null;
            ShareService.instance.saveText(
              selectedText,
              suggestedFilename:
                  'assistant-message-${DateTime.now().toIso8601String().split('T').first}.md',
              sharePositionOrigin: origin,
            );
          },
        ),
      ],
    );
  }
}
