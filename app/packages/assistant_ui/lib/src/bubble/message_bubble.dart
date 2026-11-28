import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import 'message_body.dart';
import 'typing_indicator.dart';

/// One chat message: its alignment, bubble decoration, grouping, failed
/// indicator, typing indicator and content, with slots for everything that
/// does I/O or belongs to a sibling feature.
///
/// The bubble fetches, plays, shares and downloads nothing itself. The caller
/// supplies:
///
/// - [attachments] and [audio]: above the text of a user message (audio
///   replaces the text row, so a voice message's transcript is the caller's
///   concern); below the content of an assistant message, attachments first.
/// - [extensions]: widgets inside the bubble, below everything else, for
///   generative UI.
/// - [actions]: the row below the bubble.
/// - [contextMenuBuilder] for the user's text and [selectionToolbarBuilder]
///   for the assistant's markdown: platform-adaptive selection toolbars.
///
/// Assistant content renders through [MessageBody], so markdown plugins, links
/// and images come from the nearest `ChatMarkdownScope`.
class MessageBubble extends StatelessWidget {
  const MessageBubble({
    super.key,
    required this.message,
    required this.senderLabel,
    this.isGrouped = false,
    this.attachments,
    this.audio,
    this.extensions = const [],
    this.actions,
    this.contextMenuBuilder,
    this.selectionToolbarBuilder,
  });

  final ChatMessage message;

  /// Who sent the message, as the screen reader announces it: "You" for the
  /// current user and the active Persona's name for assistant messages.
  ///
  /// It opens the bubble's semantics label, which the delivery status follows
  /// (for example "Ada, stopped").
  final String senderLabel;

  /// Whether the previous message has the same author; shrinks the top margin.
  final bool isGrouped;
  final Widget? attachments;
  final Widget? audio;
  final List<Widget> extensions;
  final Widget? actions;

  /// The context menu of the user's selectable text.
  final EditableTextContextMenuBuilder? contextMenuBuilder;

  /// The toolbar of selected assistant markdown.
  final MessageSelectionToolbarBuilder? selectionToolbarBuilder;

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    final colorScheme = Theme.of(context).colorScheme;
    final isFailed = message.status == MessageStatus.failed;

    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Column(
        crossAxisAlignment: isUser
            ? CrossAxisAlignment.end
            : CrossAxisAlignment.start,
        children: [
          Semantics(
            container: true,
            explicitChildNodes: true,
            label: _semanticsLabel,
            child: Container(
              key: const Key('message_bubble_surface'),
              margin: EdgeInsets.only(top: isGrouped ? 2 : 8, bottom: 2),
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
              constraints: const BoxConstraints(maxWidth: 640),
              decoration: BoxDecoration(
                color: isUser
                    ? colorScheme.primary
                    : colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.only(
                  topLeft: const Radius.circular(16),
                  topRight: const Radius.circular(16),
                  bottomLeft: isUser
                      ? const Radius.circular(16)
                      : const Radius.circular(4),
                  bottomRight: isUser
                      ? const Radius.circular(4)
                      : const Radius.circular(16),
                ),
                border: isFailed
                    ? Border.all(color: colorScheme.error, width: 1.5)
                    : null,
              ),
              child: Column(
                crossAxisAlignment: isUser
                    ? CrossAxisAlignment.end
                    : CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  ...(isUser
                      ? _userContent(colorScheme, isFailed)
                      : _assistantContent()),
                  ...extensions,
                ],
              ),
            ),
          ),
          ?actions,
        ],
      ),
    );
  }

  /// The sender, followed by the delivery status the bubble is in.
  String get _semanticsLabel => '$senderLabel${_statusSuffix(message)}';

  static String _statusSuffix(ChatMessage message) {
    if (message.isStreaming) return ', responding';
    return switch (message.status) {
      MessageStatus.sending => ', sending',
      MessageStatus.failed => ', failed',
      MessageStatus.stopped => ', stopped',
      MessageStatus.ok => '',
    };
  }

  List<Widget> _userContent(ColorScheme colorScheme, bool isFailed) => [
    ?attachments,
    if (message.viaForm) _viaFormMarker(colorScheme),
    if (audio != null)
      audio!
    // Hide the text row when the content is blank and attachments provide the
    // visual.
    else if (message.content.trim().isNotEmpty)
      Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isFailed)
            Padding(
              padding: const EdgeInsets.only(right: 6),
              child: Icon(
                Icons.error_outline,
                size: 16,
                color: colorScheme.error,
              ),
            ),
          Flexible(
            child: SelectableText(
              message.content,
              style: TextStyle(color: colorScheme.onPrimary),
              contextMenuBuilder: contextMenuBuilder,
            ),
          ),
        ],
      ),
  ];

  /// "Submitted via form" marks a message the server built for a surface
  /// action, so the words in the bubble are not the user's own.
  Widget _viaFormMarker(ColorScheme colorScheme) => Padding(
    padding: const EdgeInsets.only(bottom: 4),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.dynamic_form_outlined,
          size: 14,
          color: colorScheme.onPrimary.withValues(alpha: 0.8),
        ),
        const SizedBox(width: 4),
        Text(
          'Submitted via form',
          style: TextStyle(
            fontSize: 11,
            color: colorScheme.onPrimary.withValues(alpha: 0.8),
          ),
        ),
      ],
    ),
  );

  List<Widget> _assistantContent() => [
    // Animated dots while waiting for the first token.
    if (message.isStreaming && message.content.isEmpty) const TypingIndicator(),
    MessageBody(
      content: message.content,
      tokenStream: message.tokenStream,
      isStreaming: message.isStreaming,
      selectionToolbarBuilder: selectionToolbarBuilder,
    ),
    ?attachments,
    ?audio,
  ];
}
