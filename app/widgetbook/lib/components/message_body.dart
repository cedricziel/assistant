import 'package:assistant_ui/assistant_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _markdown = '''
## Summary

The body renders **markdown** through the thread's scope.

- first point
- second point with `inline code`

```dart
final answer = 42;
```
''';

final messageBodyComponent = WidgetbookComponent(
  name: 'MessageBody',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => MessageBody(
        content: context.knobs.string(
          label: 'content',
          initialValue: _markdown,
          maxLines: 8,
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'image without a scope',
      builder: (context) =>
          const MessageBody(content: '![logo](https://example.com/logo.png)'),
    ),
    WidgetbookUseCase(
      name: 'empty',
      builder: (context) => const MessageBody(content: ''),
    ),
  ],
);
