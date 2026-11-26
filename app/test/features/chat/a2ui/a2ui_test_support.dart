import 'dart:typed_data';

import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:assistant_app/features/chat/a2ui/attachment_image_loader.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:material_ui/material_ui.dart';

import '../../../support/golden.dart';

/// The catalog id the server stamps on every surface.
const fixtureCatalogId = kAssistantCatalogId;

/// A `createSurface` + `updateComponents` pair for a Text and a Button.
List<Map<String, Object?>> textAndButtonMessages({
  String surfaceId = 's1',
  bool sendDataModel = false,
  Map<String, Object?>? dataModel,
}) => [
  {
    'version': '1.0',
    'createSurface': {
      'surfaceId': surfaceId,
      'catalogId': fixtureCatalogId,
      'sendDataModel': sendDataModel,
    },
  },
  if (dataModel != null)
    {
      'version': '1.0',
      'updateDataModel': {
        'surfaceId': surfaceId,
        'path': '/',
        'value': dataModel,
      },
    },
  {
    'version': '1.0',
    'updateComponents': {
      'surfaceId': surfaceId,
      'components': [
        {
          'id': 'root',
          'component': 'Column',
          'children': ['greeting', 'go'],
        },
        {'id': 'greeting', 'component': 'Text', 'text': 'Hello, surface'},
        {
          'id': 'go',
          'component': 'Button',
          'child': 'go_label',
          'variant': 'primary',
          'action': {
            'event': {
              'name': 'submit',
              'context': {'who': 'me'},
            },
          },
        },
        {'id': 'go_label', 'component': 'Text', 'text': 'Go'},
      ],
    },
  },
];

/// A `createSurface` followed by one `updateComponents` for [components].
List<Map<String, Object?>> surfaceMessages(
  List<Map<String, Object?>> components, {
  Map<String, Object?>? dataModel,
  bool sendDataModel = false,
}) => [
  {
    'version': '1.0',
    'createSurface': {
      'surfaceId': 's1',
      'catalogId': fixtureCatalogId,
      'sendDataModel': sendDataModel,
    },
  },
  if (dataModel != null)
    {
      'version': '1.0',
      'updateDataModel': {'surfaceId': 's1', 'path': '/', 'value': dataModel},
    },
  {
    'version': '1.0',
    'updateComponents': {'surfaceId': 's1', 'components': components},
  },
];

/// Wraps [child] like the app does: a Material app whose theme reaches the
/// framework widgets the stock A2UI components are built on through the
/// [LegacyDesignBridge].
Widget host(Widget child, {Brightness brightness = Brightness.light}) =>
    MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: goldenTheme(brightness),
      builder: (context, child) => LegacyDesignBridge(child: child!),
      home: Scaffold(body: Center(child: child)),
    );

/// An [AttachmentImageLoader] that returns a 1x1 PNG and records its calls.
class FakeAttachmentImageLoader implements AttachmentImageLoader {
  final List<String> loaded = [];

  @override
  Future<Uint8List> load(String attachmentId) async {
    loaded.add(attachmentId);
    return _onePixelPng;
  }
}

final Uint8List _onePixelPng = Uint8List.fromList(const [
  0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, //
  0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
  0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
  0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
  0x00, 0x05, 0xFE, 0x02, 0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00,
  0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
]);
