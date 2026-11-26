@Tags(['golden'])
library;

import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../../../support/golden.dart';
import 'a2ui_test_support.dart';

// Card, Button, TextField, ChoicePicker and DateTimeInput through the
// material bridge (openspec/changes/generative-ui-a2ui PR 14.5).

final _messages = surfaceMessages(
  [
    {'id': 'root', 'component': 'Card', 'child': 'col'},
    {
      'id': 'col',
      'component': 'Column',
      'children': ['name', 'color', 'when', 'go'],
    },
    {
      'id': 'name',
      'component': 'TextField',
      'label': 'Name',
      'value': {'@path': '/name'},
    },
    {
      'id': 'color',
      'component': 'ChoicePicker',
      'label': 'Colour',
      'variant': 'mutuallyExclusive',
      'options': [
        {'label': 'Red', 'value': 'red'},
        {'label': 'Blue', 'value': 'blue'},
      ],
      'value': {'@path': '/color'},
    },
    {
      'id': 'when',
      'component': 'DateTimeInput',
      'label': 'When',
      'enableDate': true,
      'value': {'@path': '/when'},
    },
    {
      'id': 'go',
      'component': 'Button',
      'child': 'label',
      'variant': 'primary',
      'action': {
        'event': {'name': 'go'},
      },
    },
    {'id': 'label', 'component': 'Text', 'text': 'Go'},
  ],
  dataModel: {
    'name': 'Ada',
    'color': ['blue'],
    'when': '2026-11-26',
  },
);

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('assistant surface, ${brightness.name}', (tester) async {
      await pumpGolden(
        tester,
        A2uiSurfaceView(
          messages: _messages,
          catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
        ),
        size: const Size(400, 420),
        brightness: brightness,
      );
      await tester.pump();

      await expectGolden('assistant_surface_${brightness.name}');
    });
  }
}
