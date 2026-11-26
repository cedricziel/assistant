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

Widget _menu(String filter) => Padding(
  padding: const EdgeInsets.all(12),
  child: SlashCommandMenu(
    commands: _commands,
    filter: filter,
    onSelect: (_) {},
  ),
);

final slashCommandMenuComponent = WidgetbookComponent(
  name: 'SlashCommandMenu',
  useCases: [
    WidgetbookUseCase(name: 'all commands', builder: (context) => _menu('')),
    WidgetbookUseCase(name: 'filtered', builder: (context) => _menu('mo')),
  ],
);
