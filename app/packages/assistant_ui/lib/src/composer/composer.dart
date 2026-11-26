import 'package:material_ui/material_ui.dart';

import '../models/command_entry.dart';
import '../models/pending_attachment.dart';
import '../theme/assistant_ui_colors.dart';
import '../theme/assistant_ui_spacing.dart';
import 'attachment_tray.dart';
import 'slash_command_menu.dart';

/// Builds the composer's text input.
///
/// The composer hands over its [controller], [focusNode], the [placeholder]
/// to show and an [onSubmit] callback that the input calls on its submit
/// action. The app passes `AdaptiveTextField` here; without a builder the
/// composer renders a neutral Material `TextField`. Whatever is returned
/// should carry `Key('message_input')`.
typedef ComposerInputBuilder = Widget Function(
  BuildContext context,
  TextEditingController controller,
  FocusNode focusNode,
  String placeholder,
  VoidCallback onSubmit,
);

/// The chat input row: slash-command menu, queue-depth badge, pending
/// attachments, the input, and the attach, voice, send and stop affordances.
///
/// The composer emits every intent by callback and performs no I/O: it does
/// not pick files, record audio or call an API. The app supplies the voice
/// affordance through [voiceButton] and the text field through [inputBuilder].
class Composer extends StatefulWidget {
  const Composer({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.isSending,
    this.isStopping = false,
    required this.pendingQueueCount,
    required this.pendingAttachments,
    required this.onSend,
    required this.onStop,
    required this.onPickImage,
    required this.onRemoveAttachment,
    this.commands = const [],
    this.showVoice = false,
    this.voiceButton,
    this.inputBuilder,
  });

  final TextEditingController controller;
  final FocusNode focusNode;

  /// A turn is running: the stop affordance replaces send.
  final bool isSending;

  /// A stop was requested and is not confirmed yet: stop is disabled.
  final bool isStopping;

  /// Messages waiting in the send queue; the badge shows when above zero.
  final int pendingQueueCount;
  final List<PendingAttachment> pendingAttachments;
  final VoidCallback onSend;
  final VoidCallback onStop;
  final VoidCallback onPickImage;
  final void Function(int index) onRemoveAttachment;

  /// Slash commands offered while the text starts with `/`.
  final List<CommandEntry> commands;

  /// Whether the server accepts voice messages; [voiceButton] shows only then.
  final bool showVoice;

  /// The voice affordance, shown where the mic is when [showVoice] is true
  /// and no turn is running.
  final Widget? voiceButton;

  /// Replaces the neutral default input; see [ComposerInputBuilder].
  final ComposerInputBuilder? inputBuilder;

  @override
  State<Composer> createState() => _ComposerState();
}

class _ComposerState extends State<Composer> {
  bool _showMenu = false;
  String _commandFilter = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onTextChanged);
  }

  @override
  void didUpdateWidget(covariant Composer oldWidget) {
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

    if (shouldShow != _showMenu || filter != _commandFilter) {
      setState(() {
        _showMenu = shouldShow;
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
      // No-arg command: set text and submit through the normal send path.
      widget.controller.text = '/${cmd.name}';
      setState(() => _showMenu = false);
      widget.onSend();
    }
  }

  Widget _buildInput(BuildContext context) {
    final placeholder = widget.pendingAttachments.isNotEmpty
        ? 'Add a caption...'
        : 'Type a message...';
    final builder = widget.inputBuilder;
    if (builder != null) {
      return builder(
        context,
        widget.controller,
        widget.focusNode,
        placeholder,
        widget.onSend,
      );
    }
    return TextField(
      key: const Key('message_input'),
      controller: widget.controller,
      focusNode: widget.focusNode,
      decoration: InputDecoration(
        hintText: placeholder,
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 14,
          vertical: 10,
        ),
      ),
      cursorColor: Theme.of(context).colorScheme.primary,
      minLines: 1,
      maxLines: 6,
      textInputAction: TextInputAction.send,
      onSubmitted: (_) => widget.onSend(),
    );
  }

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.of(context).padding.bottom;
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        // Slash-command menu, shown when the text starts with /.
        if (_showMenu)
          Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AssistantUiSpacing.md,
            ),
            child: SlashCommandMenu(
              key: const Key('command_autocomplete_popup'),
              commands: widget.commands,
              filter: _commandFilter,
              onSelect: _onCommandSelected,
            ),
          ),
        // Queue depth badge, visible when messages are waiting.
        if (widget.pendingQueueCount > 0)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(
              horizontal: AssistantUiSpacing.lg,
              vertical: AssistantUiSpacing.xs,
            ),
            color: AssistantUiColors.warning.withValues(alpha: 0.12),
            child: Row(
              children: [
                const Icon(
                  Icons.hourglass_top_rounded,
                  size: 14,
                  color: AssistantUiColors.warning,
                ),
                const SizedBox(width: AssistantUiSpacing.xs),
                Text(
                  '${widget.pendingQueueCount} message${widget.pendingQueueCount == 1 ? '' : 's'} queued',
                  key: const Key('queue_depth_badge'),
                  style: const TextStyle(
                    fontSize: 12,
                    color: AssistantUiColors.warning,
                  ),
                ),
              ],
            ),
          ),
        if (widget.pendingAttachments.isNotEmpty)
          AttachmentTray(
            attachments: widget.pendingAttachments,
            onRemove: widget.onRemoveAttachment,
          ),
        Container(
          padding: EdgeInsets.fromLTRB(
            AssistantUiSpacing.md,
            AssistantUiSpacing.sm,
            AssistantUiSpacing.md,
            AssistantUiSpacing.md + bottomInset,
          ),
          decoration: BoxDecoration(
            border: Border(top: BorderSide(color: colorScheme.outlineVariant)),
          ),
          child: Row(
            children: [
              if (!widget.isSending)
                IconButton(
                  key: const Key('attach_image_button'),
                  icon: const Icon(Icons.image_outlined),
                  tooltip: 'Attach image',
                  onPressed: widget.onPickImage,
                ),
              if (widget.showVoice &&
                  !widget.isSending &&
                  widget.voiceButton != null)
                widget.voiceButton!,
              if (!widget.isSending)
                const SizedBox(width: AssistantUiSpacing.xs),
              Expanded(child: _buildInput(context)),
              const SizedBox(width: AssistantUiSpacing.sm),
              if (widget.isSending)
                IconButton.filled(
                  key: const Key('stop_button'),
                  onPressed: widget.isStopping ? null : widget.onStop,
                  tooltip: widget.isStopping ? 'Stopping…' : null,
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
