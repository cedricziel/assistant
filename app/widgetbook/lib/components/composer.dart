import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

const _commands = [
  CommandEntry(
    name: 'help',
    description: 'Show available commands',
    category: 'General',
  ),
  CommandEntry(
    name: 'model',
    description: 'Switch the active model',
    category: 'Session',
    args: [CommandArg(name: 'name', required: true)],
  ),
  CommandEntry(
    name: 'clear',
    description: 'Clear the conversation',
    category: 'Session',
  ),
];

/// Owns the controller and focus node a [Composer] needs.
class _ComposerHost extends StatefulWidget {
  const _ComposerHost({
    this.isSending = false,
    this.isStopping = false,
    this.queued = 0,
    this.commands = const [],
    this.text = '',
  });

  final bool isSending;
  final bool isStopping;
  final int queued;
  final List<CommandEntry> commands;
  final String text;

  @override
  State<_ComposerHost> createState() => _ComposerHostState();
}

class _ComposerHostState extends State<_ComposerHost> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.text,
  );
  final FocusNode _focusNode = FocusNode();

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Composer(
    controller: _controller,
    focusNode: _focusNode,
    isSending: widget.isSending,
    isStopping: widget.isStopping,
    pendingQueueCount: widget.queued,
    pendingAttachments: const [],
    commands: widget.commands,
    onSend: () {},
    onStop: () {},
    onPickImage: () {},
    onRemoveAttachment: (_) {},
  );
}

final composerComponent = WidgetbookComponent(
  name: 'Composer',
  useCases: [
    WidgetbookUseCase(
      name: 'idle',
      builder: (context) => const _ComposerHost(),
    ),
    WidgetbookUseCase(
      name: 'sending',
      builder: (context) => const _ComposerHost(isSending: true),
    ),
    WidgetbookUseCase(
      name: 'stopping',
      builder: (context) =>
          const _ComposerHost(isSending: true, isStopping: true),
    ),
    WidgetbookUseCase(
      name: 'queue badge',
      builder: (context) => const _ComposerHost(queued: 2),
    ),
    WidgetbookUseCase(
      name: 'slash menu open',
      builder: (context) => const _ComposerHost(commands: _commands, text: '/'),
    ),
  ],
);
