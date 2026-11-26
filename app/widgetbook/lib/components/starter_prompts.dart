import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

const _prompts = [
  'Plan my week around the meetings on my calendar',
  'Review the open pull requests',
  'Summarise the news from this morning',
];

Widget _at(double width, Widget child) => Padding(
  padding: const EdgeInsets.all(16),
  child: Align(
    alignment: Alignment.topLeft,
    child: SizedBox(width: width, child: child),
  ),
);

final starterPromptsComponent = WidgetbookComponent(
  name: 'StarterPrompts',
  useCases: [
    WidgetbookUseCase(
      name: 'narrow',
      builder: (context) =>
          _at(400, StarterPrompts(prompts: _prompts, onSelected: (_) {})),
    ),
    WidgetbookUseCase(
      name: 'wide',
      builder: (context) =>
          _at(720, StarterPrompts(prompts: _prompts, onSelected: (_) {})),
    ),
  ],
);
