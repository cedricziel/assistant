// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14.3.
import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_app/api/space_header_interceptor.dart';
import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_image.dart';
import 'package:assistant_app/features/chat/a2ui/attachment_image_loader.dart';
import 'package:dio/dio.dart';
// ignore: deprecated_member_use
import 'package:flutter/material.dart' as framework;
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'a2ui_test_support.dart';

const _attachmentId = '3f2b8c1e-1d4a-4c8e-9a57-0b6d1f2e7a90';

/// A Dio adapter that records requests and answers with the test PNG.
class _RecordingAdapter implements HttpClientAdapter {
  final List<RequestOptions> requests = [];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(options);
    final bytes = await FakeAttachmentImageLoader().load('x');
    return ResponseBody.fromBytes(bytes, 200);
  }

  @override
  void close({bool force = false}) {}
}

Widget _imageSurface(
  AttachmentImageLoader loader,
  Object url, {
  Map<String, Object?>? dataModel,
  List<Map<String, Object?>> extra = const [],
}) => host(
  A2uiSurfaceView(
    catalog: assistantCatalog(imageLoader: loader),
    messages: surfaceMessages([
      {
        'id': 'root',
        'component': 'Column',
        'children': ['img', for (final c in extra) c['id']! as String],
      },
      {'id': 'img', 'component': 'Image', 'url': url, 'description': 'A chart'},
      ...extra,
    ], dataModel: dataModel),
  ),
);

void main() {
  testWidgets('attachment image renders with auth headers', (tester) async {
    final adapter = _RecordingAdapter();
    final dio = Dio(BaseOptions(baseUrl: 'https://assistant.test'))
      ..httpClientAdapter = adapter
      ..interceptors.addAll([
        InterceptorsWrapper(
          onRequest: (o, h) {
            o.headers['Authorization'] = 'Bearer tok';
            h.next(o);
          },
        ),
        SpaceHeaderInterceptor(spaceId: () => 'space-7'),
      ]);

    await tester.runAsync(() async {
      await tester.pumpWidget(
        _imageSurface(
          DioAttachmentImageLoader(dio),
          '/api/attachments/$_attachmentId',
        ),
      );
      // Real time for the request and the bytes to arrive.
      for (var i = 0; i < 100; i++) {
        await Future<void>.delayed(const Duration(milliseconds: 20));
        await tester.pump();
        if (find.byType(Image).evaluate().isNotEmpty) break;
      }
    });

    expect(adapter.requests, hasLength(1));
    final request = adapter.requests.single;
    expect(request.path, '/api/attachments/$_attachmentId');
    expect(request.headers['Authorization'], 'Bearer tok');
    expect(request.headers[spaceIdHeader], 'space-7');
    expect(find.byType(Image), findsOneWidget);
    expect(find.byKey(placeholderKey), findsNothing);
  });

  testWidgets('a malformed attachment path is not loaded', (tester) async {
    final loader = FakeAttachmentImageLoader();
    await tester.pumpWidget(_imageSurface(loader, '/api/attachments/../x'));
    await tester.pump();

    expect(loader.loaded, isEmpty);
    expect(find.byKey(placeholderKey), findsOneWidget);
  });

  testWidgets('any https host renders', (tester) async {
    await tester.pumpWidget(
      _imageSurface(
        FakeAttachmentImageLoader(),
        'https://never-seen.example/pic.png',
      ),
    );
    await tester.pump();

    final image = tester.widget<Image>(find.byType(Image));
    expect(
      (image.image as NetworkImage).url,
      'https://never-seen.example/pic.png',
    );
  });

  for (final url in [
    'http://example.org/a.png',
    'data:image/png;base64,AAAA',
    'javascript:alert(1)',
    'file:///etc/passwd',
  ]) {
    testWidgets('$url is blocked', (tester) async {
      final loader = FakeAttachmentImageLoader();
      await tester.pumpWidget(_imageSurface(loader, url));
      await tester.pump();

      expect(find.byType(Image), findsNothing);
      expect(find.byKey(placeholderKey), findsOneWidget);
      expect(loader.loaded, isEmpty);
    });
  }

  testWidgets('a bound url the user changes to data: becomes a placeholder', (
    tester,
  ) async {
    await tester.pumpWidget(
      _imageSurface(
        FakeAttachmentImageLoader(),
        {'@path': '/pic'},
        dataModel: {'pic': 'https://ok.example/a.png'},
        extra: [
          {
            'id': 'field',
            'component': 'TextField',
            'label': 'URL',
            'value': {'@path': '/pic'},
          },
        ],
      ),
    );
    await tester.pump();
    expect(find.byType(Image), findsOneWidget);

    await tester.enterText(
      find.byType(framework.TextField),
      'data:image/png;base64,${base64Encode([1, 2, 3])}',
    );
    await tester.pump();

    expect(find.byType(Image), findsNothing);
    expect(find.byKey(placeholderKey), findsOneWidget);
  });
}
