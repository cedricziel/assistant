import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

final toolStatusIconComponent = WidgetbookComponent(
  name: 'ToolStatusIcon',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => Padding(
        padding: const EdgeInsets.all(16),
        child: ToolStatusIcon(
          status: enumKnob(
            context,
            label: 'status',
            values: ToolCallStatus.values,
            initial: ToolCallStatus.ok,
          ),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'all statuses',
      builder: (context) => Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          spacing: 16,
          children: [
            for (final status in ToolCallStatus.values)
              ToolStatusIcon(status: status),
          ],
        ),
      ),
    ),
  ],
);
