// RED→GREEN: openspec/changes/generative-ui-a2ui PR 15.4 — the timeline
// adapter renders a surface entry through SurfaceFrame. PR 17.4 adds the
// pending marker and the action callback that reaches the host.

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/surface_action.dart';
import 'package:assistant_app/features/chat/chat_timeline_entry.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../../features/chat/a2ui/a2ui_test_support.dart';

ChatMessage _entry({
  bool answerable = true,
  MessageStatus status = MessageStatus.ok,
  List<Map<String, Object?>> messages = const [],
}) {
  return ChatMessage(
    id: 'surface-surface-1',
    role: 'assistant',
    content: '',
    status: status,
    timelineType: TimelineEntryType.surface,
    surface: SurfaceRecord(
      surfaceId: 'surface-1',
      anchorMessageId: 'message-1',
      anchorCallIndex: 0,
      title: 'Weather',
      fallbackText: 'Sunny, 21 °C',
      status: SurfaceStatus.active,
      answerable: answerable,
      messages: messages,
    ),
  );
}

Widget _wrap(Widget child, {ApiClient? api}) {
  return ProviderScope(
    overrides: [
      apiClientProvider.overrideWithValue(api),
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

  testWidgets('a wide transcript caps the surface at the message width', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_wrap(ChatTimelineEntry(message: _entry())));

    final frame = find.byType(SurfaceFrame);
    expect(
      tester.getSize(frame).width,
      640,
      reason: 'a surface must not overhang the message column',
    );
    expect(
      tester.getTopLeft(frame).dx,
      0,
      reason: 'the column is left-aligned',
    );
  });

  testWidgets('a narrow transcript leaves the surface full width', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(412, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_wrap(ChatTimelineEntry(message: _entry())));

    expect(tester.getSize(find.byType(SurfaceFrame)).width, 412);
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

  testWidgets('a pending surface shows progress', (tester) async {
    await tester.pumpWidget(
      _wrap(ChatTimelineEntry(message: _entry(status: MessageStatus.sending))),
    );

    expect(find.byType(LinearProgressIndicator), findsOneWidget);
  });

  testWidgets('a Button action reaches the host with its context', (
    tester,
  ) async {
    final api = ApiClient(baseUrl: 'http://fake', token: 'fake');
    SurfaceRecord? fromSurface;
    SurfaceActionSubmission? submitted;
    await tester.pumpWidget(
      _wrap(
        ChatTimelineEntry(
          message: _entry(messages: textAndButtonMessages()),
          onSurfaceAction: (surface, action) {
            fromSurface = surface;
            submitted = action;
          },
        ),
        api: api,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Go'));
    await tester.pump();

    expect(fromSurface?.surfaceId, 'surface-1');
    expect(submitted?.surfaceId, 'surface-1');
    expect(submitted?.name, 'submit');
    expect(submitted?.context, {'who': 'me'});
  });
}
