import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _viewport({required int count}) => SizedBox(
  height: 360,
  child: ThreadViewport(
    itemCount: count,
    revision: 0,
    itemBuilder: (context, index) => Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Text('Message $index'),
    ),
  ),
);

final threadViewportComponent = WidgetbookComponent(
  name: 'ThreadViewport',
  useCases: [
    WidgetbookUseCase(
      name: 'at bottom',
      builder: (context) => _viewport(count: 4),
    ),
    WidgetbookUseCase(
      name: 'long thread',
      builder: (context) => _viewport(count: 60),
    ),
  ],
);
