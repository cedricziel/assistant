import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

Widget _composer() => const Padding(
  padding: EdgeInsets.all(12),
  child: TextField(
    decoration: InputDecoration(
      hintText: 'Message',
      border: OutlineInputBorder(),
    ),
  ),
);

Widget _thread() => ThreadViewport(
  itemCount: 4,
  revision: 0,
  itemBuilder: (context, index) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 8),
    child: Text('Message $index'),
  ),
);

Widget _frame(ChatThreadLayout layout) => SizedBox(height: 420, child: layout);

final chatThreadLayoutComponent = WidgetbookComponent(
  name: 'ChatThreadLayout',
  useCases: [
    WidgetbookUseCase(
      name: 'thread and composer',
      builder: (context) =>
          _frame(ChatThreadLayout(thread: _thread(), composer: _composer())),
    ),
    WidgetbookUseCase(
      name: 'banners and progress',
      builder: (context) => _frame(
        ChatThreadLayout(
          banners: [
            ThreadErrorBanner(message: 'Server unreachable', onDismiss: () {}),
            const LinearProgressIndicator(),
          ],
          thread: _thread(),
          aboveComposer: [
            TurnProgressCard(
              status: TurnStatusSnapshot(
                runId: 'run-1',
                lastEventKind: TurnEventKind.toolResult,
                lastEventAt: _now.subtract(const Duration(seconds: 2)),
                currentToolName: 'web-search',
              ),
              onSkip: () {},
              now: () => _now,
            ),
          ],
          composer: _composer(),
        ),
      ),
    ),
  ],
);
