import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/models/server_capabilities.dart';
import 'chat_provider.dart';
import 'message_bubble.dart';
import 'models_provider.dart';

/// Wires a message's branching actions to [ChatNotifier], keeping the
/// timeline's item builder out of it.
///
/// Edit opens an inline [MessageEditor] in place of the bubble, over the
/// message's attachments so they stay visible while the text changes. Send
/// calls [ChatNotifier.editMessage], Regenerate calls
/// [ChatNotifier.regenerate], and the branch picker calls
/// [ChatNotifier.switchBranch] with the sibling version's id.
///
/// Edit and Regenerate are offered while no turn works, including while a turn
/// only waits for input, whose pending interaction the server cancels. A
/// switch in flight keeps every picker busy. Optimistic messages (a sending
/// bubble or the streaming placeholder) are not persisted and cannot branch.
class ChatMessageActionsWiring extends ConsumerStatefulWidget {
  const ChatMessageActionsWiring({
    super.key,
    required this.message,
    required this.senderLabel,
    required this.capabilities,
    required this.fetchMessageAudio,
    this.isGrouped = false,
    this.onRetry,
    this.onStop,
    this.isStopping = false,
    this.showResponseTiming = false,
    this.imageBaseUrl,
    this.imageAuthToken,
    this.onFeedbackChanged,
    this.feedbackComment,
  });

  final ChatMessage message;
  final String senderLabel;
  final bool isGrouped;
  final ServerCapabilities capabilities;
  final Future<({Uint8List bytes, String mimeType})?> Function()
  fetchMessageAudio;
  final VoidCallback? onRetry;
  final VoidCallback? onStop;
  final bool isStopping;
  final bool showResponseTiming;
  final String? imageBaseUrl;
  final String? imageAuthToken;

  /// Receives a tapped feedback rating, or `null` when it is cleared.
  final ValueChanged<MessageRating?>? onFeedbackChanged;

  /// The optional feedback comment field below the actions.
  final Widget? feedbackComment;

  @override
  ConsumerState<ChatMessageActionsWiring> createState() =>
      _ChatMessageActionsWiringState();
}

class _ChatMessageActionsWiringState
    extends ConsumerState<ChatMessageActionsWiring> {
  bool _editing = false;

  @override
  Widget build(BuildContext context) {
    final chat = ref.watch(chatProvider).value;
    final notifier = ref.read(chatProvider.notifier);
    final awaitingInput = chat?.currentTurnStatus?.awaitingLabel != null;
    final turnWorking = (chat?.isSending ?? false) && !awaitingInput;
    final busy = turnWorking || (chat?.isSwitchingBranch ?? false);
    final isPersisted = _isPersisted(widget.message);
    final models =
        ref.watch(modelsProvider).asData?.value ?? const <RegenerateModel>[];

    if (_editing) return _editor(notifier);

    return ChatMessageBubbleAdapter(
      message: widget.message,
      senderLabel: widget.senderLabel,
      isGrouped: widget.isGrouped,
      capabilities: widget.capabilities,
      fetchMessageAudio: widget.fetchMessageAudio,
      onRetry: widget.onRetry,
      onStop: widget.onStop,
      isStopping: widget.isStopping,
      showResponseTiming: widget.showResponseTiming,
      imageBaseUrl: widget.imageBaseUrl,
      imageAuthToken: widget.imageAuthToken,
      onFeedbackChanged: widget.onFeedbackChanged,
      feedbackComment: widget.feedbackComment,
      isPersisted: isPersisted,
      turnWorking: busy,
      onEdit: isPersisted ? () => setState(() => _editing = true) : null,
      onRegenerate: isPersisted
          ? () => unawaited(notifier.regenerate(widget.message.id))
          : null,
      onRegenerateWith: isPersisted
          ? (model) =>
                unawaited(notifier.regenerate(widget.message.id, model: model))
          : null,
      regenerateModels: models,
      onSwitchBranch: (messageId) =>
          unawaited(notifier.switchBranch(messageId)),
    );
  }

  Widget _editor(ChatNotifier notifier) {
    return Align(
      alignment: Alignment.centerRight,
      child: Container(
        constraints: const BoxConstraints(maxWidth: 640),
        margin: const EdgeInsets.only(top: 8, bottom: 2),
        child: MessageEditor(
          initialText: widget.message.content,
          attachments: [
            for (final attachment in widget.message.attachments)
              Chip(
                key: Key('edit_attachment_${attachment.id}'),
                avatar: const Icon(Icons.attach_file),
                label: Text(attachment.filename),
              ),
          ],
          onCancel: () => setState(() => _editing = false),
          onSubmit: (text) {
            setState(() => _editing = false);
            unawaited(notifier.editMessage(widget.message.id, text));
          },
        ),
      ),
    );
  }
}

/// A server id is a UUID. The optimistic user bubble uses a `user-` prefixed
/// id, the streaming assistant a fixed placeholder, and a stopped reply the
/// server never persisted a local `assistant-<millis>` id.
bool _isPersisted(ChatMessage message) =>
    !message.isStreaming && _serverId.hasMatch(message.id);

final _serverId = RegExp(
  r'^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-'
  r'[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
);
