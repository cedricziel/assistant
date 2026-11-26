import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _framed(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

final suggestionChipsComponent = WidgetbookComponent(
  name: 'SuggestionChips',
  useCases: [
    WidgetbookUseCase(
      name: 'one',
      builder: (context) => _framed(
        SuggestionChips(prompts: const ['Summarise this'], onSelected: (_) {}),
      ),
    ),
    WidgetbookUseCase(
      name: 'three',
      builder: (context) => _framed(
        SuggestionChips(
          prompts: const [
            'Summarise this',
            'Draft a reply',
            'Explain in more detail',
          ],
          onSelected: (_) {},
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'long prompt',
      builder: (context) => _framed(
        SuggestionChips(
          prompts: [
            'Walk me through every step of the migration, including the '
                'rollback plan and what to monitor afterwards',
            'Thanks',
          ],
          onSelected: (_) {},
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'disabled',
      builder: (context) => _framed(
        SuggestionChips(
          prompts: const ['Summarise this', 'Draft a reply'],
          onSelected: (_) {},
          enabled: false,
        ),
      ),
    ),
  ],
);
