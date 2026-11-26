import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final threadEmptyStateComponent = WidgetbookComponent(
  name: 'ThreadEmptyState',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) => const ThreadEmptyState(),
    ),
    WidgetbookUseCase(
      name: 'with starters',
      builder: (context) => ThreadEmptyState(
        starters: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: StarterPrompts(
            prompts: const [
              'Plan my week around the meetings on my calendar',
              'Review the open pull requests',
            ],
            onSelected: (_) {},
          ),
        ),
      ),
    ),
  ],
);
