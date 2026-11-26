// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14b.2.
import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'a2ui_test_support.dart';

const _message = 'Does not match';

/// A TextField whose only check is `regex(/v, pattern)`, with [value] bound.
Widget _field(String pattern, String value, List<Object> errors) => host(
  A2uiSurfaceView(
    catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
    onError: errors.add,
    messages: surfaceMessages(
      [
        {
          'id': 'root',
          'component': 'TextField',
          'label': 'Code',
          'value': {'@path': '/v'},
          '@checks': [
            {
              'condition': {
                '@call': 'regex',
                'args': {
                  'value': {'@path': '/v'},
                  'pattern': pattern,
                },
              },
              'message': _message,
            },
          ],
        },
      ],
      dataModel: {'v': value},
    ),
  ),
);

void main() {
  testWidgets('input within the cap is evaluated', (tester) async {
    final errors = <Object>[];
    await tester.pumpWidget(_field(r'^a*$', 'a' * 1000, errors));
    await tester.pump();

    expect(find.text(_message), findsNothing);
    expect(errors, isEmpty);
  });

  testWidgets('input of 1,001 characters fails without a match', (
    tester,
  ) async {
    final errors = <Object>[];
    await tester.pumpWidget(_field(r'^a*$', 'a' * 1001, errors));
    await tester.pump();

    expect(find.text(_message), findsOneWidget);
    expect(errors, isEmpty);
  });

  testWidgets('5,000 characters fail without evaluating the pattern', (
    tester,
  ) async {
    final errors = <Object>[];
    // The library rejects this pattern when it compiles it, which raises an
    // error. A clean failing check proves the pattern never ran.
    await tester.pumpWidget(_field(r'(a+)+$', 'a' * 5000, errors));
    await tester.pump();

    expect(find.text(_message), findsOneWidget);
    expect(errors, isEmpty, reason: 'the pattern must not be compiled');
  });

  testWidgets('the cap counts code points, not UTF-16 units', (tester) async {
    final errors = <Object>[];
    // 600 emoji are 1,200 UTF-16 units but 600 code points.
    await tester.pumpWidget(_field(r'^.*$', '😀' * 600, errors));
    await tester.pump();

    expect(find.text(_message), findsNothing);
  });
}
