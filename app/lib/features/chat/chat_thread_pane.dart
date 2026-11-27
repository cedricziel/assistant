import 'dart:typed_data';

import 'package:assistant_ui/assistant_ui.dart';

import '../../api/models/interaction_answer.dart';
import '../../api/models/server_capabilities.dart';
import '../../shared/platform/widgets.dart';
import 'chat_empty_starters.dart';
import 'chat_provider.dart';
import 'chat_timeline_entry.dart';
import 'follow_up_chips.dart';
import 'message_bubble.dart';
import 'message_feedback_provider.dart';
import 'reconnect_banner.dart';

/// The conversation surface of [ChatScreen]: the thread, its banners, the
/// widgets above the composer and the composer itself.
///
/// The screen keeps the provider reads, routing and I/O; this widget takes
/// their results and builds the pure element tree. Callbacks are the app's
/// notifier calls, so nothing here touches Riverpod.
class ChatThreadPane extends StatelessWidget {
  const ChatThreadPane({
    super.key,
    required this.chatState,
    required this.banners,
    required this.threadWidth,
    required this.threadController,
    required this.revision,
    required this.capabilities,
    required this.pendingAttachments,
    required this.commands,
    required this.inputController,
    required this.inputFocus,
    required this.activePersonaName,
    required this.onSend,
    required this.onStop,
    required this.onPickImage,
    required this.onRemoveAttachment,
    required this.onShowQueuedMessage,
    required this.onInteractionAnswer,
    required this.onRetry,
    required this.fetchMessageAudio,
    required this.feedbackState,
    required this.onFeedbackChanged,
    required this.onSubmitFeedbackComment,
    required this.onDismissFeedbackComment,
    this.showResponseTiming = false,
    this.showSkip = true,
    this.showVoice = false,
    this.voiceButton,
    this.imageBaseUrl,
    this.imageAuthToken,
  });

  final ChatState chatState;
  final List<Widget> banners;
  final double threadWidth;
  final ThreadViewportController threadController;
  final int revision;
  final ServerCapabilities capabilities;
  final List<PendingAttachment> pendingAttachments;
  final List<CommandEntry> commands;
  final TextEditingController inputController;
  final FocusNode inputFocus;

  /// The active Persona's name; it labels the assistant's bubbles.
  final String activePersonaName;

  /// Whether finished replies show their response timing.
  final bool showResponseTiming;
  final bool showSkip;
  final bool showVoice;
  final Widget? voiceButton;
  final String? imageBaseUrl;
  final String? imageAuthToken;

  final VoidCallback onSend;
  final VoidCallback onStop;
  final VoidCallback onPickImage;

  /// Called when the user removes the attachment at [index].
  final void Function(int index) onRemoveAttachment;
  final void Function(PendingMessage message, int index) onShowQueuedMessage;
  final void Function(String interactionId, InteractionAnswer answer)
  onInteractionAnswer;
  final void Function(ChatMessage message) onRetry;
  final Future<({Uint8List bytes, String mimeType})?> Function(ChatMessage)
  fetchMessageAudio;

  /// Per-message feedback state: pending votes and open comment fields.
  final MessageFeedbackState feedbackState;
  final void Function(ChatMessage message, MessageRating? rating)
  onFeedbackChanged;
  final void Function(ChatMessage message, String comment)
  onSubmitFeedbackComment;
  final void Function(ChatMessage message) onDismissFeedbackComment;

  @override
  Widget build(BuildContext context) {
    return ChatThreadLayout(
      banners: banners,
      thread: ThreadDensityScope.fromWidth(
        width: threadWidth,
        child: chatState.messages.isEmpty && chatState.pendingQueue.isEmpty
            ? const ChatEmptyStarters()
            : ThreadViewport(
                controller: threadController,
                revision: revision,
                itemCount:
                    chatState.messages.length + chatState.pendingQueue.length,
                itemBuilder: (context, index) => _entry(context, index),
              ),
      ),
      aboveComposer: [
        // Brief banner shown while attemptReconnect is recovering an
        // interrupted stream after a resume. Renders nothing on routine
        // foreground transitions.
        const ReconnectBanner(),
        // Follow-up prompts after a finished turn.
        const FollowUpChips(),
        // In-flight turn progress card — renders nothing when no turn is
        // active, an activity label otherwise, a stall label after 30s of
        // silence.
        TurnProgressCard(
          status: chatState.currentTurnStatus,
          showSkip: showSkip,
          awaitingLabel: chatState.currentTurnStatus?.awaitingLabel,
          // The SSE stream surfaces the server-side cancellation as
          // `agent_error`, which the notifier reconciles.
          onSkip: onStop,
        ),
      ],
      composer: _composer(context),
    );
  }

  Widget _entry(BuildContext context, int index) {
    // Render queued ghost bubbles after the committed messages, in send
    // order. The actively-streaming message is already in `messages` (it was
    // popped from the queue when its stream started), so ghost bubbles
    // represent only the truly-waiting items.
    if (index >= chatState.messages.length) {
      final queueIdx = index - chatState.messages.length;
      final pending = chatState.pendingQueue[queueIdx];
      return QueuedMessageBubble(
        message: pending,
        onLongPress: () => onShowQueuedMessage(pending, queueIdx),
      );
    }

    final message = chatState.messages[index];
    if (message.timelineType != TimelineEntryType.message) {
      return ChatTimelineEntry(
        message: message,
        conversationId: chatState.conversationId,
        entryState: timelineEntryState(message),
        onInteractionAnswer: onInteractionAnswer,
      );
    }

    final previous = index > 0 ? chatState.messages[index - 1] : null;
    final isGrouped =
        previous != null &&
        previous.role == message.role &&
        !previous.isStreaming;
    return ChatMessageBubbleAdapter(
      message: message,
      senderLabel: message.isUser ? 'You' : activePersonaName,
      isGrouped: isGrouped,
      capabilities: capabilities,
      showResponseTiming: showResponseTiming,
      onFeedbackChanged: (rating) => onFeedbackChanged(message, rating),
      feedbackComment: feedbackState.commentFields[message.id] == null
          ? null
          : FeedbackCommentField(
              key: ValueKey(
                'feedback_comment_${message.id}_${feedbackState.commentFields[message.id]}',
              ),
              initialText: message.feedback?.comment,
              sending: feedbackState.pending.containsKey(message.id),
              onSubmit: (comment) => onSubmitFeedbackComment(message, comment),
              onDismiss: () => onDismissFeedbackComment(message),
            ),
      onRetry: message.status == MessageStatus.failed
          ? () => onRetry(message)
          : null,
      isStopping: chatState.isStopping,
      onStop: onStop,
      fetchMessageAudio: () => fetchMessageAudio(message),
      imageBaseUrl: imageBaseUrl,
      imageAuthToken: imageAuthToken,
    );
  }

  Widget _composer(BuildContext context) {
    return Composer(
      controller: inputController,
      focusNode: inputFocus,
      isSending: chatState.isSending,
      isStopping: chatState.isStopping,
      pendingQueueCount: chatState.pendingQueue.length,
      pendingAttachments: pendingAttachments,
      commands: commands,
      showVoice: showVoice,
      voiceButton: voiceButton,
      inputBuilder: (context, controller, focusNode, placeholder, onSubmit) =>
          AdaptiveTextField(
            key: const Key('message_input'),
            controller: controller,
            focusNode: focusNode,
            placeholder: placeholder,
            cursorColor: Theme.of(context).colorScheme.primary,
            contentPadding: const EdgeInsets.symmetric(
              horizontal: 14,
              vertical: 10,
            ),
            minLines: 1,
            maxLines: 6,
            textInputAction: TextInputAction.send,
            onSubmitted: (_) => onSubmit(),
          ),
      onSend: onSend,
      onStop: onStop,
      onPickImage: onPickImage,
      onRemoveAttachment: onRemoveAttachment,
    );
  }
}
