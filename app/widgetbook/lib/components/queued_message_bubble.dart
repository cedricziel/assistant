import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:widgetbook/widgetbook.dart';

final queuedMessageBubbleComponent = WidgetbookComponent(
  name: 'QueuedMessageBubble',
  useCases: [
    WidgetbookUseCase(
      name: 'short',
      builder: (context) => QueuedMessageBubble(
        message: const PendingMessage(
          text: 'Summarise this thread',
          conversationId: 'c',
        ),
        onLongPress: () {},
      ),
    ),
    WidgetbookUseCase(
      name: 'long',
      builder: (context) => QueuedMessageBubble(
        message: const PendingMessage(
          text:
              'Walk me through every step of the migration, including the '
              'rollback plan and what to monitor afterwards, then summarise '
              'the risks',
          conversationId: 'c',
        ),
        onLongPress: () {},
      ),
    ),
  ],
);
