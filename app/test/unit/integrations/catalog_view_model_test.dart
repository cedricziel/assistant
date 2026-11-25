import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/catalog_view_model.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fakes

class _Call {
  _Call({this.q, this.category, this.limit, this.cursor});

  final String? q;
  final String? category;
  final int? limit;
  final String? cursor;
}

class _FakeIntegrationsApi extends IntegrationsApi {
  _FakeIntegrationsApi(this._handler) : super(Dio(), standardSerializers);

  final Future<CatalogEntryListResponse> Function(_Call call) _handler;
  final List<_Call> calls = [];

  @override
  Future<Response<CatalogEntryListResponse>> listCatalogEntries({
    String? q,
    String? category,
    int? limit,
    String? cursor,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final call = _Call(q: q, category: category, limit: limit, cursor: cursor);
    calls.add(call);
    final data = await _handler(call);
    return Response(
      requestOptions: RequestOptions(path: '/api/integrations/catalog'),
      statusCode: 200,
      data: data,
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient(this._integrations)
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeIntegrationsApi _integrations;

  @override
  IntegrationsApi get integrations => _integrations;
}

CatalogEntrySummary _entry(
  String id, {
  List<String> categories = const [],
  CatalogAuthKindResponse kind = CatalogAuthKindResponse.none,
}) {
  return CatalogEntrySummary(
    (b) => b
      ..id = 'curated/$id'
      ..name = id
      ..description = 'About $id'
      ..domain = '$id.example.com'
      ..connectUrl = 'https://$id.example.com/mcp'
      ..categories = ListBuilder<String>(categories)
      ..auth = CatalogAuthResponse((a) => a..kind = kind).toBuilder(),
  );
}

CatalogEntryListResponse _page(
  List<CatalogEntrySummary> items, {
  String? nextCursor,
}) {
  return CatalogEntryListResponse(
    (b) => b
      ..items = ListBuilder<CatalogEntrySummary>(items)
      ..nextCursor = nextCursor,
  );
}

({ProviderContainer container, _FakeIntegrationsApi api}) _setup(
  Future<CatalogEntryListResponse> Function(_Call call) handler,
) {
  final api = _FakeIntegrationsApi(handler);
  final container = ProviderContainer(
    overrides: [apiClientProvider.overrideWithValue(_FakeApiClient(api))],
  );
  // autoDispose provider: keep it alive for the duration of the test.
  container.listen(catalogViewModelProvider, (_, _) {});
  return (container: container, api: api);
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('CatalogViewModel', () {
    test('is loading until the first page arrives', () async {
      final gate = Completer<CatalogEntryListResponse>();
      final t = _setup((_) => gate.future);
      addTearDown(t.container.dispose);

      expect(t.container.read(catalogViewModelProvider).isLoading, isTrue);

      gate.complete(_page([_entry('linear')]));
      final state = await t.container.read(catalogViewModelProvider.future);
      expect(state.items.map((e) => e.name), ['linear']);
    });

    test('loads the first page with the default page size', () async {
      final t = _setup((_) async => _page([_entry('linear')]));
      addTearDown(t.container.dispose);

      await t.container.read(catalogViewModelProvider.future);

      expect(t.api.calls, hasLength(1));
      expect(t.api.calls.single.q, isNull);
      expect(t.api.calls.single.category, isNull);
      expect(t.api.calls.single.cursor, isNull);
      expect(t.api.calls.single.limit, catalogPageSize);
    });

    test('exposes the next cursor and sorted unique categories', () async {
      final t = _setup(
        (_) async => _page([
          _entry('linear', categories: ['project-management', 'dev']),
          _entry('github', categories: ['dev']),
        ], nextCursor: 'abc'),
      );
      addTearDown(t.container.dispose);

      final state = await t.container.read(catalogViewModelProvider.future);

      expect(state.nextCursor, 'abc');
      expect(state.categories, ['dev', 'project-management']);
      expect(state.error, isNull);
      expect(state.isLoading, isFalse);
    });

    test('reports an empty catalog as success without items', () async {
      final t = _setup((_) async => _page([]));
      addTearDown(t.container.dispose);

      final state = await t.container.read(catalogViewModelProvider.future);

      expect(state.items, isEmpty);
      expect(state.error, isNull);
      expect(state.nextCursor, isNull);
    });

    test('surfaces a failed first load as an error state', () async {
      final t = _setup((_) async => throw Exception('boom'));
      addTearDown(t.container.dispose);

      final state = await t.container.read(catalogViewModelProvider.future);

      expect(state.error, contains('boom'));
      expect(state.items, isEmpty);
    });

    test('describes an HTTP failure without the Dio boilerplate', () async {
      final t = _setup(
        (_) async => throw DioException.badResponse(
          statusCode: 500,
          requestOptions: RequestOptions(path: '/api/integrations/catalog'),
          response: Response(
            requestOptions: RequestOptions(path: '/api/integrations/catalog'),
            statusCode: 500,
          ),
        ),
      );
      addTearDown(t.container.dispose);

      final state = await t.container.read(catalogViewModelProvider.future);

      expect(state.error, 'The server answered with status 500.');
    });

    test('describes an unreachable server', () async {
      final t = _setup(
        (_) async => throw DioException.connectionError(
          requestOptions: RequestOptions(path: '/api/integrations/catalog'),
          reason: 'socket',
        ),
      );
      addTearDown(t.container.dispose);

      final state = await t.container.read(catalogViewModelProvider.future);

      expect(state.error, 'The server could not be reached.');
    });

    test('retry reloads after an error', () async {
      var fail = true;
      final t = _setup((_) async {
        if (fail) throw Exception('boom');
        return _page([_entry('linear')]);
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);

      fail = false;
      await t.container.read(catalogViewModelProvider.notifier).retry();

      final state = t.container.read(catalogViewModelProvider).requireValue;
      expect(state.error, isNull);
      expect(state.items.map((e) => e.name), ['linear']);
    });

    test('returns an empty state when not connected', () async {
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);
      container.listen(catalogViewModelProvider, (_, _) {});

      final state = await container.read(catalogViewModelProvider.future);

      expect(state.items, isEmpty);
      expect(state.error, isNull);
    });

    test('debounces search input by 300 ms', () {
      fakeAsync((async) {
        final t = _setup((_) async => _page([_entry('linear')]));
        async.flushMicrotasks();
        final notifier = t.container.read(catalogViewModelProvider.notifier);
        expect(t.api.calls, hasLength(1), reason: 'initial load');

        notifier.setQuery('l');
        async.elapse(const Duration(milliseconds: 200));
        notifier.setQuery('lin');
        async.elapse(const Duration(milliseconds: 299));
        async.flushMicrotasks();
        expect(t.api.calls, hasLength(1), reason: 'still inside the window');

        async.elapse(const Duration(milliseconds: 2));
        async.flushMicrotasks();
        expect(t.api.calls, hasLength(2));
        expect(t.api.calls.last.q, 'lin');
        expect(t.api.calls.last.cursor, isNull);
        expect(
          t.container.read(catalogViewModelProvider).requireValue.query,
          'lin',
        );
        t.container.dispose();
      });
    });

    test('clearing the search sends no q parameter', () {
      fakeAsync((async) {
        final t = _setup((_) async => _page([_entry('linear')]));
        async.flushMicrotasks();
        final notifier = t.container.read(catalogViewModelProvider.notifier);

        notifier.setQuery('lin');
        async.elapse(catalogSearchDebounce);
        async.flushMicrotasks();
        notifier.setQuery('  ');
        async.elapse(catalogSearchDebounce);
        async.flushMicrotasks();

        expect(t.api.calls.last.q, isNull);
        t.container.dispose();
      });
    });

    test('ignores a response that a newer query has superseded', () async {
      final slow = Completer<CatalogEntryListResponse>();
      final t = _setup((call) {
        if (call.category == 'slow') return slow.future;
        return Future.value(_page([_entry('fresh')]));
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);
      final notifier = t.container.read(catalogViewModelProvider.notifier);

      final stale = notifier.selectCategory('slow');
      await notifier.selectCategory(null);
      slow.complete(_page([_entry('stale')]));
      await stale;

      final state = t.container.read(catalogViewModelProvider).requireValue;
      expect(state.items.map((e) => e.name), ['fresh']);
      expect(state.category, isNull);
    });

    test('filters by category and keeps the chips stable', () async {
      final t = _setup((call) async {
        if (call.category == 'dev') {
          return _page([
            _entry('github', categories: ['dev']),
          ]);
        }
        return _page([
          _entry('github', categories: ['dev']),
          _entry('slack', categories: ['chat']),
        ]);
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);
      final notifier = t.container.read(catalogViewModelProvider.notifier);

      await notifier.selectCategory('dev');

      var state = t.container.read(catalogViewModelProvider).requireValue;
      expect(t.api.calls.last.category, 'dev');
      expect(state.category, 'dev');
      expect(state.items.map((e) => e.name), ['github']);
      expect(state.categories, ['chat', 'dev']);

      await notifier.selectCategory(null);

      state = t.container.read(catalogViewModelProvider).requireValue;
      expect(t.api.calls.last.category, isNull);
      expect(state.category, isNull);
      expect(state.items, hasLength(2));
    });

    test('clearFilters drops both the query and the category', () async {
      final t = _setup((_) async => _page([_entry('github')]));
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);
      final notifier = t.container.read(catalogViewModelProvider.notifier);
      await notifier.selectCategory('dev');

      await notifier.clearFilters();

      final state = t.container.read(catalogViewModelProvider).requireValue;
      expect(t.api.calls.last.q, isNull);
      expect(t.api.calls.last.category, isNull);
      expect(state.category, isNull);
      expect(state.query, isEmpty);
    });

    test('loadMore appends the next page using the cursor', () async {
      final t = _setup((call) async {
        if (call.cursor == 'c1') {
          return _page([_entry('slack')]);
        }
        return _page([_entry('github')], nextCursor: 'c1');
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);

      await t.container.read(catalogViewModelProvider.notifier).loadMore();

      final state = t.container.read(catalogViewModelProvider).requireValue;
      expect(t.api.calls.last.cursor, 'c1');
      expect(state.items.map((e) => e.name), ['github', 'slack']);
      expect(state.nextCursor, isNull);
      expect(state.isLoadingMore, isFalse);
    });

    test('loadMore keeps the current query and category', () async {
      final t = _setup(
        (call) async => call.cursor == null
            ? _page([_entry('github')], nextCursor: 'c1')
            : _page([_entry('gitlab')]),
      );
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);
      final notifier = t.container.read(catalogViewModelProvider.notifier);
      await notifier.selectCategory('dev');

      await notifier.loadMore();

      expect(t.api.calls.last.category, 'dev');
      expect(t.api.calls.last.cursor, 'c1');
    });

    test('loadMore does nothing on the last page', () async {
      final t = _setup((_) async => _page([_entry('github')]));
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);

      await t.container.read(catalogViewModelProvider.notifier).loadMore();

      expect(t.api.calls, hasLength(1));
    });

    test('loadMore failure keeps the items and flags the failure', () async {
      final t = _setup((call) async {
        if (call.cursor != null) throw Exception('offline');
        return _page([_entry('github')], nextCursor: 'c1');
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);

      await t.container.read(catalogViewModelProvider.notifier).loadMore();

      final state = t.container.read(catalogViewModelProvider).requireValue;
      expect(state.items.map((e) => e.name), ['github']);
      expect(state.loadMoreFailed, isTrue);
      expect(state.nextCursor, 'c1');
      expect(state.isLoadingMore, isFalse);
    });

    test('ignores concurrent loadMore calls', () async {
      final gate = Completer<CatalogEntryListResponse>();
      final t = _setup((call) {
        if (call.cursor == 'c1') return gate.future;
        return Future.value(_page([_entry('github')], nextCursor: 'c1'));
      });
      addTearDown(t.container.dispose);
      await t.container.read(catalogViewModelProvider.future);
      final notifier = t.container.read(catalogViewModelProvider.notifier);

      final first = notifier.loadMore();
      await notifier.loadMore();
      gate.complete(_page([_entry('slack')]));
      await first;

      expect(t.api.calls.where((c) => c.cursor == 'c1'), hasLength(1));
    });
  });
}
