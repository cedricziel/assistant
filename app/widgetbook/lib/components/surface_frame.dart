import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

/// Placeholder for the rendered A2UI tree: a field and a button.
const _form = Column(
  mainAxisSize: MainAxisSize.min,
  crossAxisAlignment: CrossAxisAlignment.start,
  children: [
    TextField(decoration: InputDecoration(labelText: 'Destination')),
    SizedBox(height: 12),
    FilledButton(onPressed: null, child: Text('Send')),
  ],
);

Widget _frame({
  SurfaceStatus status = SurfaceStatus.active,
  bool answerable = true,
  bool hasError = false,
  bool pending = false,
  bool awaitingInput = false,
  Widget? content = _form,
}) => Padding(
  padding: const EdgeInsets.all(16),
  child: SurfaceFrame(
    title: 'Trip details',
    status: status,
    answerable: answerable,
    fallbackText: 'Where would you like to go, and when?',
    hasError: hasError,
    pending: pending,
    awaitingInput: awaitingInput,
    content: content,
  ),
);

final surfaceFrameComponent = WidgetbookComponent(
  name: 'SurfaceFrame',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) => _frame(
        status: enumKnob(
          context,
          label: 'status',
          values: SurfaceStatus.values,
          initial: SurfaceStatus.active,
        ),
        answerable: context.knobs.boolean(
          label: 'answerable',
          initialValue: true,
        ),
        hasError: context.knobs.boolean(label: 'hasError'),
        pending: context.knobs.boolean(label: 'pending'),
        awaitingInput: context.knobs.boolean(label: 'awaitingInput'),
      ),
    ),
    WidgetbookUseCase(name: 'active', builder: (context) => _frame()),
    WidgetbookUseCase(
      name: 'awaiting input',
      builder: (context) => _frame(awaitingInput: true),
    ),
    WidgetbookUseCase(
      name: 'pending',
      builder: (context) => _frame(pending: true),
    ),
    WidgetbookUseCase(
      name: 'submitted',
      builder: (context) =>
          _frame(status: SurfaceStatus.submitted, answerable: false),
    ),
    WidgetbookUseCase(
      name: 'closed',
      builder: (context) => _frame(answerable: false),
    ),
    WidgetbookUseCase(
      name: 'error',
      builder: (context) => _frame(hasError: true),
    ),
    WidgetbookUseCase(
      name: 'fallback only',
      builder: (context) => _frame(content: null),
    ),
  ],
);
