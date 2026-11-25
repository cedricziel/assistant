import 'package:flutter/services.dart';

import '../../shared/platform/widgets.dart';
import '../../api/models/server_capabilities.dart';
import 'attachment_provider.dart';
import 'command_autocomplete.dart';
import 'commands_provider.dart';
import '../../shared/theme/assistant_colors.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'image_utils.dart';
import 'voice_recorder_button.dart';

class ChatInputRow extends StatefulWidget {
  const ChatInputRow({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.isSending,
    required this.pendingQueueCount,
    required this.pendingAttachments,
    required this.capabilities,
    required this.onSend,
    required this.onStop,
    required this.onVoiceRecorded,
    required this.onPickImage,
    required this.onRemoveAttachment,
    required this.onPasteImage,
    this.commands = const [],
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isSending;
  final int pendingQueueCount;
  final List<PendingAttachment> pendingAttachments;
  final ServerCapabilities capabilities;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final void Function(Uint8List bytes, String mimeType) onVoiceRecorded;
  final VoidCallback onPickImage;
  final void Function(int index) onRemoveAttachment;
  final void Function(Uint8List bytes) onPasteImage;
  final List<CommandEntry> commands;

  @override
  State<ChatInputRow> createState() => _ChatInputRowState();
}

class _ChatInputRowState extends State<ChatInputRow> {
  bool _showAutocomplete = false;
  String _commandFilter = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onTextChanged);
  }

  @override
  void didUpdateWidget(covariant ChatInputRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_onTextChanged);
      widget.controller.addListener(_onTextChanged);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onTextChanged);
    super.dispose();
  }

  void _onTextChanged() {
    final text = widget.controller.text;
    final shouldShow = text.startsWith('/') && widget.commands.isNotEmpty;
    final filter = shouldShow ? text.substring(1).split(' ').first : '';

    if (shouldShow != _showAutocomplete || filter != _commandFilter) {
      setState(() {
        _showAutocomplete = shouldShow;
        _commandFilter = filter;
      });
    }
  }

  void _onCommandSelected(CommandEntry cmd) {
    if (cmd.hasArgs) {
      // Fill the input with the command and a trailing space for the arg.
      widget.controller.text = '/${cmd.name} ';
      widget.controller.selection = TextSelection.fromPosition(
        TextPosition(offset: widget.controller.text.length),
      );
    } else {
      // No-arg command: set text and submit through normal send path.
      widget.controller.text = '/${cmd.name}';
      setState(() => _showAutocomplete = false);
      widget.onSend();
    }
  }

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.of(context).padding.bottom;
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Command autocomplete popup — shown when text starts with /.
        if (_showAutocomplete)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: CommandAutocompletePopup(
              key: const Key('command_autocomplete_popup'),
              commands: widget.commands,
              filter: _commandFilter,
              onSelect: _onCommandSelected,
            ),
          ),
        // Queue depth badge — visible when messages are waiting.
        if (widget.pendingQueueCount > 0)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(
              horizontal: AssistantSpacing.lg,
              vertical: AssistantSpacing.xs,
            ),
            color: AssistantColors.warning.withValues(alpha: 0.12),
            child: Row(
              children: [
                Icon(
                  Icons.hourglass_top_rounded,
                  size: 14,
                  color: AssistantColors.warning,
                ),
                const SizedBox(width: AssistantSpacing.xs),
                Text(
                  '${widget.pendingQueueCount} message${widget.pendingQueueCount == 1 ? '' : 's'} queued',
                  key: const Key('queue_depth_badge'),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AssistantColors.warning,
                  ),
                ),
              ],
            ),
          ),
        // Pending attachment thumbnails.
        if (widget.pendingAttachments.isNotEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 0),
            child: SizedBox(
              height: 72,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: widget.pendingAttachments.length,
                separatorBuilder: (_, _) => const SizedBox(width: 8),
                itemBuilder: (context, index) {
                  final attachment = widget.pendingAttachments[index];
                  return Stack(
                    children: [
                      ClipRRect(
                        borderRadius: BorderRadius.circular(8),
                        child: isImageMimeType(attachment.mimeType)
                            ? Image.memory(
                                attachment.bytes,
                                width: 64,
                                height: 64,
                                fit: BoxFit.cover,
                              )
                            : Container(
                                width: 64,
                                height: 64,
                                color: colorScheme.surfaceContainerHighest,
                                child: Column(
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    Icon(
                                      iconForMime(attachment.mimeType),
                                      size: 28,
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                                    const SizedBox(height: 2),
                                    Text(
                                      attachment.filename.length > 8
                                          ? '${attachment.filename.substring(0, 8)}…'
                                          : attachment.filename,
                                      style: Theme.of(context)
                                          .textTheme
                                          .labelSmall
                                          ?.copyWith(
                                            color: colorScheme.onSurfaceVariant,
                                          ),
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ],
                                ),
                              ),
                      ),
                      Positioned(
                        top: -4,
                        right: -4,
                        child: IconButton(
                          key: Key('remove_attachment_$index'),
                          icon: const Icon(Icons.cancel, size: 18),
                          onPressed: () => widget.onRemoveAttachment(index),
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                          color: colorScheme.error,
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        Container(
          padding: EdgeInsets.fromLTRB(12, 8, 12, 12 + bottomInset),
          decoration: BoxDecoration(
            border: Border(top: BorderSide(color: colorScheme.outlineVariant)),
          ),
          child: Row(
            children: [
              // Image picker button.
              if (!widget.isSending)
                IconButton(
                  key: const Key('attach_image_button'),
                  icon: const Icon(Icons.image_outlined),
                  tooltip: 'Attach image',
                  onPressed: widget.onPickImage,
                ),
              // Voice recorder button — shown when server supports voice send.
              if (widget.capabilities.voiceSend && !widget.isSending)
                VoiceRecorderButton(
                  onRecordingComplete: widget.onVoiceRecorded,
                  onError: (err) {
                    ScaffoldMessenger.of(context)
                        .showSnackBar(SnackBar(content: Text(err)));
                  },
                ),
              if (!widget.isSending) const SizedBox(width: 4),
              Expanded(
                child: KeyboardListener(
                  focusNode: FocusNode(),
                  onKeyEvent: (event) {
                    // Detect Ctrl+V / Cmd+V paste with image data.
                    if (event is KeyDownEvent &&
                        event.logicalKey == LogicalKeyboardKey.keyV &&
                        (HardwareKeyboard.instance.isControlPressed ||
                            HardwareKeyboard.instance.isMetaPressed)) {
                      Clipboard.getData('image/png').then((data) {
                        // ClipboardData doesn't support binary; use
                        // the text field's default paste for text.
                        // Image paste from clipboard is handled via
                        // the super_clipboard package if available.
                      });
                    }
                  },
                  child: AdaptiveTextField(
                    key: const Key('message_input'),
                    controller: widget.controller,
                    focusNode: widget.focusNode,
                    placeholder: widget.pendingAttachments.isNotEmpty
                        ? 'Add a caption...'
                        : 'Type a message...',
                    cursorColor: colorScheme.primary,
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 14,
                      vertical: 10,
                    ),
                    minLines: 1,
                    maxLines: 6,
                    textInputAction: TextInputAction.send,
                    onSubmitted: (_) => widget.onSend(),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              if (widget.isSending)
                IconButton.filled(
                  key: const Key('stop_button'),
                  onPressed: widget.onStop,
                  icon: const Icon(Icons.stop_rounded),
                )
              else
                IconButton.filled(
                  key: const Key('send_button'),
                  onPressed: widget.onSend,
                  icon: const Icon(Icons.send),
                ),
            ],
          ),
        ),
      ],
    );
  }
}
