// RED→GREEN: openspec/changes/generative-ui-a2ui PR 15.4 — the timeline
// adapter renders a surface entry through SurfaceFrame.

import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/features/chat/chat_timeline_entry.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

ChatMessage _entry({bool answerable = true}) {
  return ChatMessage(
    id: 'surface-surface-1',
    role: 'assistant',
    content: '',
    timelineType: TimelineEntryType.surface,
    surface: SurfaceRecord(
      surfaceId: 'surface-1',
      anchorMessageId: 'message-1',
      anchorCallIndex: 0,
      title: 'Weather',
      fallbackText: 'Sunny, 21 °C',
      status: SurfaceStatus.active,
      answerable: answerable,
    ),
  );
}

Widget _wrap(Widget child) {
  return ProviderScope(
    overrides: [
      apiClientProvider.overrideWithValue(null),
      uiLinkHostsProvider.overrideWithValue(const []),
    ],
    child: MaterialApp(
      home: MediaQuery(
        data: const MediaQueryData(size: Size(500, 800)),
        child: Scaffold(body: child),
      ),
    ),
  );
}

void main() {
  testWidgets('renders the frame chrome and the text fallback', (tester) async {
    await tester.pumpWidget(_wrap(ChatTimelineEntry(message: _entry())));

    expect(find.text('Weather'), findsOneWidget);
    expect(
      find.text('Sunny, 21 °C'),
      findsOneWidget,
      reason: 'without a renderer the surface reads as its fallback text',
    );
    expect(find.byType(SurfaceFrame), findsOneWidget);
  });

  testWidgets('a surface that is no longer answerable reads Closed', (
    tester,
  ) async {
    await tester.pumpWidget(
      _wrap(ChatTimelineEntry(message: _entry(answerable: false))),
    );

    expect(find.text('Closed'), findsOneWidget);
    expect(find.text('Sunny, 21 °C'), findsOneWidget);
  });
}
