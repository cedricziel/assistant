// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14.1 and 14.6.
import 'dart:io';

import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:flutter_test/flutter_test.dart';

import 'a2ui_test_support.dart';

void main() {
  test('catalog components equal the shared fixture', () {
    final fixture = File('test/fixtures/a2ui/components.txt')
        .readAsLinesSync()
        .map((l) => l.trim())
        .where((l) => l.isNotEmpty)
        .toSet();

    final catalog = assistantCatalog(imageLoader: FakeAttachmentImageLoader());

    expect(catalog.id, kAssistantCatalogId);
    expect(catalog.components.keys.toSet(), fixture);
  });

  testWidgets('a createSurface fixture renders Text and a Button', (
    tester,
  ) async {
    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: textAndButtonMessages(),
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('Hello, surface'), findsOneWidget);
    expect(find.text('Go'), findsOneWidget);
  });

  testWidgets('onAction carries the A2UI action', (tester) async {
    final actions = <SurfaceAction>[];
    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: textAndButtonMessages(),
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
          onAction: actions.add,
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('Go'));
    await tester.pump();

    expect(actions, hasLength(1));
    expect(actions.single.action.name, 'submit');
    expect(actions.single.action.surfaceId, 's1');
    expect(actions.single.action.context, {'who': 'me'});
    expect(actions.single.dataModel, isNull, reason: 'sendDataModel is off');
  });

  testWidgets('onAction adds the data model when sendDataModel is set', (
    tester,
  ) async {
    final actions = <SurfaceAction>[];
    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: textAndButtonMessages(
            sendDataModel: true,
            dataModel: {'name': 'Ada'},
          ),
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
          onAction: actions.add,
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('Go'));
    await tester.pump();

    expect(actions.single.dataModel, {'name': 'Ada'});
  });

  testWidgets('a renderer error is reported through onError', (tester) async {
    final errors = <Object>[];
    final broken = textAndButtonMessages();
    // An unknown component type is a message-processing error.
    (((broken.last['updateComponents']! as Map)['components']! as List)[1]
            as Map)['component'] =
        'Video';

    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: broken,
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
          onError: errors.add,
        ),
      ),
    );
    await tester.pump();

    expect(errors, isNotEmpty);
  });
}
