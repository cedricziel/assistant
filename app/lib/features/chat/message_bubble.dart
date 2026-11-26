import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart';

import 'svg_builder.dart';

import '../../shared/platform/widgets.dart';
import '../../services/share_service.dart';
import '../../api/models/server_capabilities.dart';
import 'audio_player_widget.dart';
import 'chat_provider.dart';
import '../../shared/theme/legacy_theme.dart';
import 'markdown_link_handler.dart';
import 'message_attachments.dart';
import 'message_audio_actions.dart';
import 'message_meta_actions.dart';
import 'typing_dots.dart';
import 'voice_message_player.dart';
import 'theme_aware_mermaid.dart';

/// A single chat message bubble: text, markdown, attachments, inline audio
/// and the meta action row below it.
class MessageBubble extends StatelessWidget {
  const MessageBubble({
    super.key,
    required this.message,
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
          Container(
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
            child: isUser
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (message.attachments.isNotEmpty)
                        MessageAttachments(
                          attachments: message.attachments,
                          isUser: isUser,
                          imageBaseUrl: imageBaseUrl,
                          imageAuthToken: imageAuthToken,
                        ),
                      // Voice message: show audio player + collapsible transcript.
                      if (message.audioBytes != null)
                        AdaptiveMediaContextMenu(
                          actions: voiceMessageActions(
                            context,
                            message.audioBytes!,
                            message.audioMimeType ?? 'audio/webm',
                          ),
                          child: VoiceMessagePlayer(
                            audioBytes: message.audioBytes!,
                            audioMimeType: message.audioMimeType,
                            transcript: message.content,
                            foregroundColor: colorScheme.onPrimary,
                          ),
                        )
                      // Hide text row when content is only whitespace /
                      // zero-width space and attachments provide the visual.
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
                                contextMenuBuilder:
                                    _textSelectionContextMenuBuilder,
                              ),
                            ),
                          ],
                        ),
                    ],
                  )
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      // Animated dots while waiting for first token.
                      if (message.isStreaming && message.content.isEmpty)
                        const TypingDots(),
                      if (message.isStreaming && message.tokenStream != null)
                        StreamMarkdown(
                          stream: message.tokenStream!,
                          styleSheet: _markdownStyleSheet(context, colorScheme),
                          useEnhancedComponents: true,
                          onTapLink: (url) =>
                              MarkdownLinkHandler(context: context).onTap(url),
                          plugins: _markdownPlugins(),
                          builderRegistry: _markdownBuilders(colorScheme),
                        )
                      else if (message.content.isNotEmpty)
                        SmoothMarkdown(
                          data: message.content,
                          styleSheet: _markdownStyleSheet(context, colorScheme),
                          selectable: true,
                          useEnhancedComponents: true,
                          onTapLink: (url) =>
                              MarkdownLinkHandler(context: context).onTap(url),
                          plugins: _markdownPlugins(),
                          builderRegistry: _markdownBuilders(colorScheme),
                        ),
                      // Attachment thumbnails (assistant-produced images).
                      if (message.attachments.isNotEmpty)
                        MessageAttachments(
                          attachments: message.attachments,
                          isUser: isUser,
                          imageBaseUrl: imageBaseUrl,
                          imageAuthToken: imageAuthToken,
                        ),
                      // Inline audio player for messages with real audio
                      // (agent intentionally produced voice via AudioReadyEvent).
                      if (message.audioId != null && !message.isStreaming)
                        AdaptiveMediaContextMenu(
                          actions: ttsAudioActions(context, fetchMessageAudio),
                          child: Padding(
                            padding: const EdgeInsets.only(top: 6),
                            child: AudioPlayerWidget(
                              fetchAudio: fetchMessageAudio,
                            ),
                          ),
                        ),
                    ],
                  ),
          ),
          // Meta action row below the bubble.
          MetaActionRow(
            message: message,
            capabilities: capabilities,
            fetchMessageAudio: fetchMessageAudio,
            onRetry: onRetry,
            onStop: onStop,
            isStopping: isStopping,
          ),
        ],
      ),
    );
  }

  static MarkdownStyleSheet _markdownStyleSheet(
    BuildContext context,
    ColorScheme colorScheme,
  ) {
    return MarkdownStyleSheet.fromTheme(legacyThemeOf(context)).copyWith(
      paragraphStyle: TextStyle(color: colorScheme.onSurface),
      inlineCodeStyle: TextStyle(
        color: colorScheme.onSurface,
        backgroundColor: colorScheme.surfaceContainerLowest,
      ),
    );
  }

  static ParserPluginRegistry _markdownPlugins() => ParserPluginRegistry()
    ..registerBlock(MermaidPlugin())
    ..registerBlock(const SvgPlugin());

  static BuilderRegistry _markdownBuilders(ColorScheme colorScheme) =>
      BuilderRegistry()
        ..register(
          'mermaid',
          ThemeAwareMermaidBuilder(
            style: mermaidStyleFromColorScheme(colorScheme),
          ),
        )
        ..register('svg', const SvgBuilder());

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
