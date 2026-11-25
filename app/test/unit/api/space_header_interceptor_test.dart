// Tests for SpaceHeaderInterceptor — data-plane calls carry `X-Space-Id`.
//
// Spec: openspec/changes/multi-tenant-data-plane/specs/web-data-plane-tenancy

import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/space_header_interceptor.dart';

class _RecordingAdapter implements HttpClientAdapter {
  final List<RequestOptions> requests = [];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    requests.add(options);
    return ResponseBody.fromBytes(
      utf8.encode('{}'),
      200,
      headers: {
        'content-type': ['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

(Dio, _RecordingAdapter) _client(String? Function() spaceId) {
  final adapter = _RecordingAdapter();
  final dio = Dio(BaseOptions(baseUrl: 'http://server'))
    ..httpClientAdapter = adapter
    ..interceptors.add(SpaceHeaderInterceptor(spaceId: spaceId));
  return (dio, adapter);
}

void main() {
  test('sends the selected space id as X-Space-Id', () async {
    final (dio, adapter) = _client(() => 'spc_a1');

    await dio.get<dynamic>('/api/conversations');

    expect(adapter.requests.single.headers['X-Space-Id'], 'spc_a1');
  });

  test('reads the selection on every request', () async {
    String? selected = 'spc_a1';
    final (dio, adapter) = _client(() => selected);

    await dio.get<dynamic>('/api/conversations');
    selected = 'spc_b2';
    await dio.get<dynamic>('/api/conversations');

    expect(adapter.requests.map((r) => r.headers['X-Space-Id']), [
      'spc_a1',
      'spc_b2',
    ]);
  });

  test('omits the header when no space is selected', () async {
    final (dio, adapter) = _client(() => null);

    await dio.get<dynamic>('/api/conversations');

    expect(adapter.requests.single.headers.containsKey('X-Space-Id'), isFalse);
  });

  test('keeps a header the call already set', () async {
    final (dio, adapter) = _client(() => 'spc_a1');

    await dio.get<dynamic>(
      '/api/conversations',
      options: Options(headers: {'X-Space-Id': 'spc_explicit'}),
    );

    expect(adapter.requests.single.headers['X-Space-Id'], 'spc_explicit');
  });
}
