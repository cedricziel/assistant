import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final WidgetbookComponent branchPickerComponent = WidgetbookComponent(
  name: 'BranchPicker',
  useCases: [
    WidgetbookUseCase(
      name: 'Default',
      builder: (context) {
        final count = context.knobs.int.slider(
          label: 'count',
          initialValue: 3,
          min: 1,
          max: 9,
        );
        final position = context.knobs.int.slider(
          label: 'index',
          initialValue: 1,
          min: 0,
          max: 8,
        );
        return Center(
          child: BranchPicker(
            index: position.clamp(0, count - 1),
            count: count,
            busy: context.knobs.boolean(label: 'busy'),
            onPrevious: () {},
            onNext: () {},
          ),
        );
      },
    ),
  ],
);
