import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/conversation_search_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fakes --------------------------------------------------------------------

/// One recorded `GET /api/conversations/search` call.
class _SearchCall {
  _SearchCall(this.q, this.cancelToken);

  final String? q;
  final CancelToken? cancelToken;
  final Completer<List<ConversationSearchSummary>> completer = Completer();
}

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  final List<_SearchCall> calls = [];

  @override
  Future<Response<BuiltList<ConversationSearchSummary>>> searchConversations({
    String? q,
    int? limit,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final call = _SearchCall(q, cancelToken);
    calls.add(call);
    final results = await call.completer.future;
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/search'),
      statusCode: 200,
      data: BuiltList<ConversationSearchSummary>(results),
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient() : super(baseUrl: 'http://fake.test', token: 'test-token');

  final StreamController<ConversationListEvent> events =
      StreamController<ConversationListEvent>.broadcast();
  final _FakeConversationsApi api = _FakeConversationsApi();

  @override
  Stream<ConversationListEvent> streamConversations({String? agentId}) =>
      events.stream;

  @override
  ConversationsApi get conversations => api;
}

ConversationSearchSummary _summary(String id, {String title = 'Trip'}) {
  return ConversationSearchSummary(
    (b) => b
      ..id = id
      ..title = title
      ..createdAt = DateTime.utc(2024, 1, 1)
      ..updatedAt = DateTime.utc(2024, 1, 1)
      ..titleSegments = ListBuilder<SearchSegment>([
        SearchSegment(
          (s) => s
            ..text = title
            ..highlighted = true,
        ),
      ])
      ..matches = ListBuilder<MessageSearchMatch>([
        MessageSearchMatch(
          (m) => m
            ..messageId = 'm-$id'
            ..role = 'assistant'
            ..createdAt = DateTime.utc(2024, 1, 1)
            ..segments = ListBuilder<SearchSegment>([
              SearchSegment(
                (s) => s
                  ..text = 'The '
                  ..highlighted = false,
              ),
              SearchSegment(
                (s) => s
                  ..text = 'Lisbon'
                  ..highlighted = true,
              ),
            ]),
        ),
      ]),
  );
}

ConversationListEntry _entry(String id, {String title = 'Trip'}) =>
    ConversationListEntry(
      id: id,
      title: title,
      createdAt: DateTime.utc(2024, 1, 1),
      updatedAt: DateTime.utc(2024, 1, 1),
    );

// -- Harness ------------------------------------------------------------------

const _debounce = Duration(milliseconds: 250);

class _Harness {
  _Harness(this.client) {
    container = ProviderContainer(
      overrides: [apiClientProvider.overrideWithValue(client)],
    );
    sub = container.listen(conversationSearchProvider, (_, _) {});
  }

  final _FakeApiClient client;
  late final ProviderContainer container;
  late final ProviderSubscription<ConversationSearchState> sub;

  _FakeConversationsApi get api => client.api;
  ConversationSearchState get state =>
      container.read(conversationSearchProvider);

  void type(String text) =>
      container.read(conversationSearchQueryProvider.notifier).set(text);

  void dispose() {
    sub.close();
    container.dispose();
  }
}

void _flush(FakeAsync async, [Duration by = _debounce]) {
  async.elapse(by);
  async.flushMicrotasks();
}

void main() {
  group('conversationSearchProvider', () {
    test('is idle while the query is empty and sends no request', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        expect(h.state, isA<ConversationSearchIdle>());

        h.type('   ');
        _flush(async);

        expect(h.state, isA<ConversationSearchIdle>());
        expect(h.api.calls, isEmpty);
      });
    });

    test('debounces typing into a single request', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('lis');
        _flush(async, const Duration(milliseconds: 100));
        h.type('lisb');
        _flush(async, const Duration(milliseconds: 249));
        expect(h.api.calls, isEmpty, reason: 'still inside the debounce');

        _flush(async, const Duration(milliseconds: 1));

        expect(h.api.calls.map((c) => c.q), ['lisb']);
        expect(h.state, isA<ConversationSearchLoading>());
      });
    });

    test('sends the trimmed query', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('  lisbon ');
        _flush(async);

        expect(h.api.calls.map((c) => c.q), ['lisbon']);
      });
    });

    test('maps results to package items', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('lisbon');
        _flush(async);
        h.api.calls.single.completer.complete([_summary('a')]);
        async.flushMicrotasks();

        final data = h.state as ConversationSearchData;
        final item = data.results.single;
        expect(item.id, 'a');
        expect(item.titleSegments.single.highlighted, isTrue);
        expect(item.matches.single.map((s) => s.text), ['The ', 'Lisbon']);
        expect(item.matches.single.last.highlighted, isTrue);
        expect(item.archivedAt, isNull);
      });
    });

    test('an empty response is data with no results', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('zzz');
        _flush(async);
        h.api.calls.single.completer.complete([]);
        async.flushMicrotasks();

        expect((h.state as ConversationSearchData).results, isEmpty);
      });
    });

    test('a failed request is an error, and retry re-sends the query', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('lisbon');
        _flush(async);
        h.api.calls.single.completer.completeError(
          DioException(
            requestOptions: RequestOptions(path: '/api/conversations/search'),
            message: 'boom',
          ),
        );
        async.flushMicrotasks();

        expect(h.state, isA<ConversationSearchError>());

        h.container.read(conversationSearchProvider.notifier).retry();
        async.flushMicrotasks();

        expect(h.api.calls.map((c) => c.q), ['lisbon', 'lisbon']);
        expect(h.state, isA<ConversationSearchLoading>());
      });
    });

    test('cancels a superseded request and discards its result', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('lis');
        _flush(async);
        final first = h.api.calls.single;

        h.type('lisb');
        _flush(async);
        final second = h.api.calls.last;

        expect(first.cancelToken?.isCancelled, isTrue);

        // The cancelled request still resolves, late.
        first.completer.complete([_summary('stale')]);
        async.flushMicrotasks();
        expect(h.state, isA<ConversationSearchLoading>());

        second.completer.complete([_summary('fresh')]);
        async.flushMicrotasks();
        final data = h.state as ConversationSearchData;
        expect(data.results.map((r) => r.id), ['fresh']);
      });
    });

    test('clearing the query cancels the request and goes idle', () {
      fakeAsync((async) {
        final h = _Harness(_FakeApiClient());
        addTearDown(h.dispose);

        h.type('lis');
        _flush(async);
        final call = h.api.calls.single;

        h.type('');
        async.flushMicrotasks();
        expect(h.state, isA<ConversationSearchIdle>());
        expect(call.cancelToken?.isCancelled, isTrue);

        call.completer.complete([_summary('late')]);
        async.flushMicrotasks();
        expect(h.state, isA<ConversationSearchIdle>());
      });
    });

    group('conversation stream', () {
      _Harness withResults(FakeAsync async, List<String> ids) {
        final h = _Harness(_FakeApiClient());
        h.type('lisbon');
        _flush(async);
        h.api.calls.single.completer.complete([
          for (final id in ids) _summary(id),
        ]);
        async.flushMicrotasks();
        return h;
      }

      test('a deleted visible result is removed without a request', () {
        fakeAsync((async) {
          final h = withResults(async, ['a', 'b']);
          addTearDown(h.dispose);

          h.client.events.add(const ConversationDeletedEvent('a'));
          _flush(async);

          final data = h.state as ConversationSearchData;
          expect(data.results.map((r) => r.id), ['b']);
          expect(h.api.calls, hasLength(1));
        });
      });

      test('an upserted visible result re-runs the query once', () {
        fakeAsync((async) {
          final h = withResults(async, ['a', 'b']);
          addTearDown(h.dispose);

          h.client.events
            ..add(ConversationUpsertedEvent(_entry('a', title: 'Renamed')))
            ..add(ConversationUpsertedEvent(_entry('b')));
          _flush(async, const Duration(milliseconds: 249));
          expect(h.api.calls, hasLength(1), reason: 're-run is debounced');

          _flush(async, const Duration(milliseconds: 1));

          expect(h.api.calls.map((c) => c.q), ['lisbon', 'lisbon']);
          expect(
            h.state,
            isA<ConversationSearchData>(),
            reason: 'the old results stay up while the query re-runs',
          );

          h.api.calls.last.completer.complete([
            _summary('a', title: 'Renamed'),
          ]);
          async.flushMicrotasks();
          final data = h.state as ConversationSearchData;
          expect(data.results.single.title, 'Renamed');
        });
      });

      test('events for conversations outside the results are ignored', () {
        fakeAsync((async) {
          final h = withResults(async, ['a']);
          addTearDown(h.dispose);

          h.client.events
            ..add(ConversationUpsertedEvent(_entry('zzz')))
            ..add(const ConversationDeletedEvent('yyy'))
            ..add(ConversationSnapshotEvent([_entry('a')]));
          _flush(async, const Duration(seconds: 1));

          expect(h.api.calls, hasLength(1));
          final data = h.state as ConversationSearchData;
          expect(data.results.map((r) => r.id), ['a']);
        });
      });
    });
  });
}
