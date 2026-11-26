@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/golden.dart';

const _size = Size(420, 360);

Widget _viewport(int count) => ThreadViewport(
  itemCount: count,
  revision: 0,
  itemBuilder: (context, index) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 8),
    child: Text('Message $index'),
  ),
);

Widget _layout() => ChatThreadLayout(
  banners: [
    ThreadErrorBanner(message: 'Server unreachable', onDismiss: () {}),
    const LinearProgressIndicator(value: 0.4),
  ],
  thread: _viewport(3),
  aboveComposer: [
    TurnProgressCard(
      status: TurnStatusSnapshot(
        runId: 'run-1',
        lastEventKind: TurnEventKind.toolResult,
        lastEventAt: kGoldenNow.subtract(const Duration(seconds: 2)),
        currentToolName: 'web-search',
      ),
      onSkip: () {},
      now: () => kGoldenNow,
    ),
  ],
  composer: const Padding(
    padding: EdgeInsets.all(12),
    child: TextField(
      decoration: InputDecoration(
        hintText: 'Message',
        border: OutlineInputBorder(),
      ),
    ),
  ),
);

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('thread_viewport_at_bottom (${brightness.name})', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        SizedBox.fromSize(size: _size, child: _viewport(40)),
        size: _size,
        brightness: brightness,
      );
      tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position
          .jumpTo(
            tester
                .state<ScrollableState>(find.byType(Scrollable))
                .position
                .maxScrollExtent,
          );
      await tester.pump(const Duration(milliseconds: 100));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/thread_viewport_at_bottom_${brightness.name}.png',
        ),
      );
    });

    testWidgets('thread_viewport_scrolled_away (${brightness.name})', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        SizedBox.fromSize(size: _size, child: _viewport(40)),
        size: _size,
        brightness: brightness,
      );
      tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position
          .jumpTo(200);
      await tester.pump(const Duration(milliseconds: 100));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/thread_viewport_scrolled_away_${brightness.name}.png',
        ),
      );
    });

    testWidgets('chat_thread_layout (${brightness.name})', (tester) async {
      await pumpGolden(
        tester,
        SizedBox.fromSize(size: _size, child: _layout()),
        size: _size,
        brightness: brightness,
      );
      await tester.pump(const Duration(milliseconds: 100));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/chat_thread_layout_${brightness.name}.png'),
      );
    });
  }
}
