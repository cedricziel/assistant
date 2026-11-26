// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14.4.
import 'dart:io';

import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:flutter_test/flutter_test.dart';

import 'a2ui_test_support.dart';

List<Map<String, Object?>> _linkButton(String url) => surfaceMessages([
  {
    'id': 'root',
    'component': 'Button',
    'child': 'label',
    'action': {
      'functionCall': {
        '@call': 'openUrl',
        'args': {'url': url},
      },
    },
  },
  {'id': 'label', 'component': 'Text', 'text': 'Open'},
]);

void main() {
  testWidgets('openUrl reaches the injected LinkOpener on a tap', (
    tester,
  ) async {
    final opened = <Uri>[];
    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: _linkButton('https://example.org/a'),
          catalog: assistantCatalog(
            imageLoader: FakeAttachmentImageLoader(),
            openUrl: opened.add,
          ),
        ),
      ),
    );
    await tester.pump();

    expect(opened, isEmpty, reason: 'rendering must not open anything');
    await tester.tap(find.text('Open'));
    await tester.pump();

    expect(opened, [Uri.parse('https://example.org/a')]);
  });

  testWidgets('without the seam a tap opens nothing and raises no error', (
    tester,
  ) async {
    final errors = <Object>[];
    await tester.pumpWidget(
      host(
        A2uiSurfaceView(
          messages: _linkButton('https://example.org/a'),
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
          onError: errors.add,
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.text('Open'));
    await tester.pump();

    expect(errors, isEmpty);
    expect(tester.takeException(), isNull);
  });

  test('no a2ui adapter file imports url_launcher before PR 14b', () {
    final offenders = [
      for (final f in Directory('lib/features/chat/a2ui').listSync())
        if (f is File && f.readAsStringSync().contains("package:url_launcher"))
          f.path,
    ];

    expect(offenders, isEmpty);
  });
}
