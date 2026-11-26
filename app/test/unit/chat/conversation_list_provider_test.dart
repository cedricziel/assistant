import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fake API client with a controllable conversation stream.

class _FakeApiClient extends ApiClient {
  _FakeApiClient({
    StreamController<ConversationListEvent>? streamController,
    _FakeConversationsApi? conversationsApi,
  }) : streamController =
           streamController ??
           StreamController<ConversationListEvent>.broadcast(),
       _conversationsApi = conversationsApi ?? _FakeConversationsApi(),
       super(baseUrl: 'http://fake.test', token: 'test-token');

  final StreamController<ConversationListEvent> streamController;
  final _FakeConversationsApi _conversationsApi;

  /// Number of times [streamConversations] has been called.
  int streamConversationsCallCount = 0;

  @override
  Stream<ConversationListEvent> streamConversations({String? agentId}) {
    streamConversationsCallCount++;
    return streamController.stream;
  }

  @override
  ConversationsApi get conversations => _conversationsApi;
}

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  final List<String> deletedIds = [];
  final List<CreateConversationRequest> createRequests = [];

  /// `(id, body)` of every PATCH, in order.
  final List<(String, UpdateConversationRequest)> updates = [];

  /// When set, [updateConversation] throws it.
  Object? updateError;

  /// `archived` argument of every [listConversations] call, in order.
  final List<bool?> listCalls = [];

  /// Body returned by [listConversations].
  List<ConversationSummary> listResponse = [];

  /// When set, [listConversations] throws it.
  Object? listError;

  /// When set, [listConversations] waits for it before answering.
  Completer<void>? listGate;

  @override
  Future<Response<BuiltList<ConversationSummary>>> listConversations({
    bool? archived,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    listCalls.add(archived);
    await listGate?.future;
    final error = listError;
    if (error != null) throw error;
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations'),
      statusCode: 200,
      data: BuiltList<ConversationSummary>(listResponse),
    );
  }

  @override
  Future<Response<ConversationSummary>> createConversation({
    required CreateConversationRequest createConversationRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    createRequests.add(createConversationRequest);
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations'),
      statusCode: 201,
      data: ConversationSummary(
        (b) => b
          ..id = 'created-1'
          ..title = 'Untitled'
          ..createdAt = DateTime(2024, 1, 1)
          ..updatedAt = DateTime(2024, 1, 1),
      ),
    );
  }

  @override
  Future<Response<ConversationSummary>> updateConversation({
    required String id,
    required UpdateConversationRequest updateConversationRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    updates.add((id, updateConversationRequest));
    final error = updateError;
    if (error != null) throw error;
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 200,
      data: ConversationSummary(
        (b) => b
          ..id = id
          ..title = updateConversationRequest.title ?? 'Test'
          ..createdAt = DateTime(2024, 1, 1)
          ..updatedAt = DateTime(2024, 1, 1),
      ),
    );
  }

  @override
  Future<Response<void>> deleteConversation({
    required String id,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    deletedIds.add(id);
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 204,
    );
  }
}

// ---------------------------------------------------------------------------
// Test helpers

/// Build a [ConversationListEntry] for testing.
ConversationListEntry _makeEntry({
  required String id,
  String title = 'Test',
  DateTime? updatedAt,
  DateTime? pinnedAt,
  DateTime? archivedAt,
}) {
  return ConversationListEntry(
    id: id,
    title: title,
    createdAt: DateTime(2024, 1, 1),
    updatedAt: updatedAt ?? DateTime(2024, 1, 1),
    pinnedAt: pinnedAt,
    archivedAt: archivedAt,
  );
}

/// Build an archived [ConversationSummary] as `?archived=true` returns it.
ConversationSummary _makeArchivedSummary(String id, DateTime archivedAt) {
  return ConversationSummary(
    (b) => b
      ..id = id
      ..title = 'Archived $id'
      ..createdAt = DateTime(2024, 1, 1)
      ..updatedAt = DateTime(2024, 1, 1)
      ..archivedAt = archivedAt,
  );
}

/// Build a [ConversationSummary] for [prependConversation] tests.
ConversationSummary _makeConv({required String id, String title = 'Test'}) {
  return ConversationSummary(
    (b) => b
      ..id = id
      ..title = title
      ..createdAt = DateTime(2024, 1, 1)
      ..updatedAt = DateTime(2024, 1, 1),
  );
}

/// Create a [ProviderContainer] with [apiClientProvider] pinned to [client].
ProviderContainer _makeContainer({ApiClient? client}) {
  return ProviderContainer(
    overrides: [apiClientProvider.overrideWithValue(client)],
  );
}

/// Wait for stream events to be delivered and the debounce timer to fire.
Future<void> _waitForDebounce() async {
  await Future<void>.delayed(const Duration(milliseconds: 350));
}

/// Keep an autoDispose provider alive for the duration of the test.
ProviderSubscription<AsyncValue<ConversationListState>> _keepAlive(
  ProviderContainer container,
) {
  return container.listen<AsyncValue<ConversationListState>>(
    conversationListProvider,
    (_, _) {},
    fireImmediately: false,
  );
}

// ---------------------------------------------------------------------------
// Mutable holder used to simulate apiClientProvider transitioning from
// null (context still loading) to a real client (context ready).

class _ApiClientHolder extends Notifier<ApiClient?> {
  @override
  ApiClient? build() => null;

  void set(ApiClient? client) => state = client;
}

final _apiClientHolderProvider = NotifierProvider<_ApiClientHolder, ApiClient?>(
  _ApiClientHolder.new,
);

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('ConversationListEntry.fromJson', () {
    Map<String, dynamic> json({String? pinnedAt}) => {
      'id': 'c1',
      'title': 'T',
      'created_at': '2024-01-01T00:00:00Z',
      'updated_at': '2024-01-02T00:00:00Z',
      'pinned_at': ?pinnedAt,
    };

    test('parses pinned_at', () {
      final entry = ConversationListEntry.fromJson(
        json(pinnedAt: '2024-03-01T00:00:00Z'),
      );
      expect(entry.pinnedAt, DateTime.utc(2024, 3, 1));
    });

    test('pinned_at is null when absent', () {
      expect(ConversationListEntry.fromJson(json()).pinnedAt, isNull);
    });

    test('parses archived_at', () {
      final entry = ConversationListEntry.fromJson({
        ...json(),
        'archived_at': '2024-04-01T00:00:00Z',
      });
      expect(entry.archivedAt, DateTime.utc(2024, 4, 1));
    });

    test('archived_at is null when absent', () {
      expect(ConversationListEntry.fromJson(json()).archivedAt, isNull);
    });
  });

  group('compareConversations', () {
    // The server orders pinned_at IS NULL, pinned_at DESC, updated_at DESC.
    // This shared fixture is listed in server order.
    final serverOrder = [
      _makeEntry(
        id: 'pin-new',
        pinnedAt: DateTime(2024, 5, 2),
        updatedAt: DateTime(2024, 1, 1),
      ),
      _makeEntry(
        id: 'pin-old',
        pinnedAt: DateTime(2024, 5, 1),
        updatedAt: DateTime(2024, 6, 1),
      ),
      _makeEntry(id: 'recent', updatedAt: DateTime(2024, 4, 1)),
      _makeEntry(id: 'older', updatedAt: DateTime(2024, 3, 1)),
    ];

    test('sorting a shuffled list reproduces the server order', () {
      final shuffled = [
        serverOrder[3],
        serverOrder[1],
        serverOrder[2],
        serverOrder[0],
      ]..sort(compareConversations);

      expect(shuffled.map((c) => c.id), serverOrder.map((c) => c.id));
    });

    test('an upserted pin moves the entry to the top', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);
      await container.read(conversationListProvider.future);

      controller.add(ConversationSnapshotEvent(serverOrder));
      await Future<void>.delayed(Duration.zero);

      controller.add(
        ConversationUpsertedEvent(
          _makeEntry(
            id: 'older',
            updatedAt: DateTime(2024, 3, 1),
            pinnedAt: DateTime(2024, 7, 1),
          ),
        ),
      );
      await _waitForDebounce();

      final ids = container
          .read(conversationListProvider)
          .value!
          .conversations
          .map((c) => c.id);
      expect(ids, ['older', 'pin-new', 'pin-old', 'recent']);
    });

    test('an upserted unpin drops the entry back by updatedAt', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);
      await container.read(conversationListProvider.future);

      controller.add(ConversationSnapshotEvent(serverOrder));
      await Future<void>.delayed(Duration.zero);

      controller.add(
        ConversationUpsertedEvent(
          _makeEntry(id: 'pin-new', updatedAt: DateTime(2024, 1, 1)),
        ),
      );
      await _waitForDebounce();

      final ids = container
          .read(conversationListProvider)
          .value!
          .conversations
          .map((c) => c.id);
      expect(ids, ['pin-old', 'recent', 'older', 'pin-new']);
    });
  });
  group('ConversationListNotifier', () {
    // -----------------------------------------------------------------------
    // Null client (context not yet loaded)
    // -----------------------------------------------------------------------

    test('returns empty state when apiClientProvider is null', () async {
      final container = _makeContainer();
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      final state = await container.read(conversationListProvider.future);
      expect(state.conversations, isEmpty);
      expect(state.error, isNull);
    });

    // -----------------------------------------------------------------------
    // Happy path — stream-driven snapshot
    // -----------------------------------------------------------------------

    test('loads conversations from stream snapshot event', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(id: 'c1', title: 'First'),
          _makeEntry(id: 'c2', title: 'Second'),
        ]),
      );
      await Future<void>.delayed(Duration.zero);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 2);
      expect(state.conversations.map((c) => c.id), containsAll(['c1', 'c2']));
      expect(state.error, isNull);
    });

    // -----------------------------------------------------------------------
    // Stream-driven upsert and delete
    // -----------------------------------------------------------------------

    test('replaces title in-place when an upserted event arrives for an existing conversation', () async {
      // This is the contract the title-generator worker depends on:
      // when the backend writes a new title (via `update_title`) and
      // broadcasts an Upserted event, the conversation list must
      // re-render with the new title without a manual refresh.
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      // Snapshot: an "Untitled" conversation lands.
      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(
            id: 'c1',
            title: 'Untitled',
            updatedAt: DateTime(2024, 1, 1),
          ),
        ]),
      );
      await Future<void>.delayed(Duration.zero);

      // Sanity: list shows the placeholder title.
      var state = container.read(conversationListProvider).value!;
      expect(state.conversations.single.title, 'Untitled');

      // Worker writes a generated title — server broadcasts the same id
      // with a newer updatedAt and the resolved title.
      controller.add(
        ConversationUpsertedEvent(
          _makeEntry(
            id: 'c1',
            title: 'Dinner Brainstorm',
            updatedAt: DateTime(2024, 1, 2),
          ),
        ),
      );
      await _waitForDebounce();

      state = container.read(conversationListProvider).value!;
      expect(
        state.conversations.length,
        1,
        reason: 'upserting an existing id must update in place, not append a duplicate',
      );
      expect(
        state.conversations.single.id,
        'c1',
        reason: 'id must be preserved across the title update',
      );
      expect(
        state.conversations.single.title,
        'Dinner Brainstorm',
        reason: 'title must reflect the worker-written value',
      );
    });

    test('applies upserted events from the stream', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(
        ConversationSnapshotEvent([_makeEntry(id: 'c1', title: 'First')]),
      );
      await Future<void>.delayed(Duration.zero);

      controller.add(
        ConversationUpsertedEvent(
          _makeEntry(id: 'c2', title: 'New', updatedAt: DateTime(2024, 2, 1)),
        ),
      );
      await _waitForDebounce();

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 2);
      expect(
        state.conversations.first.id,
        'c2',
        reason: 'newer conversation should be first (sorted by updatedAt desc)',
      );
    });

    test('applies deleted events from the stream', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(id: 'c1', title: 'Keep'),
          _makeEntry(id: 'c2', title: 'Remove'),
        ]),
      );
      await Future<void>.delayed(Duration.zero);

      controller.add(const ConversationDeletedEvent('c2'));
      await Future<void>.delayed(Duration.zero);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 1);
      expect(state.conversations.first.id, 'c1');
    });

    test('upsert replaces existing conversation and re-sorts', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(id: 'c1', title: 'First', updatedAt: DateTime(2024, 2, 1)),
          _makeEntry(
            id: 'c2',
            title: 'Second',
            updatedAt: DateTime(2024, 1, 1),
          ),
        ]),
      );
      await Future<void>.delayed(Duration.zero);
      expect(
        container.read(conversationListProvider).value!.conversations.first.id,
        'c1',
      );

      // Update c2 with a newer timestamp — should move to front
      controller.add(
        ConversationUpsertedEvent(
          _makeEntry(
            id: 'c2',
            title: 'Updated',
            updatedAt: DateTime(2024, 3, 1),
          ),
        ),
      );
      await _waitForDebounce();

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.first.id, 'c2');
      expect(state.conversations.first.title, 'Updated');
    });

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    test(
      'schedules reconnect (no immediate error) on non-auth stream error',
      () async {
        final controller = StreamController<ConversationListEvent>.broadcast();
        final client = _FakeApiClient(streamController: controller);
        final container = _makeContainer(client: client);
        addTearDown(container.dispose);
        addTearDown(controller.close);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        await container.read(conversationListProvider.future);

        controller.addError(Exception('network failure'));
        await Future<void>.delayed(Duration.zero);

        // With auto-reconnect, non-auth errors schedule a reconnect
        // instead of immediately showing an error.
        final state = container.read(conversationListProvider).value!;
        expect(
          state.error,
          isNull,
          reason: 'should not show error during reconnect attempts',
        );
      },
    );

    test('surfaces auth-specific message on ApiAuthException', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.addError(const ApiAuthException());
      await Future<void>.delayed(Duration.zero);

      final state = container.read(conversationListProvider).value!;
      expect(state.error, isNotNull);
      expect(
        state.error,
        contains('Authentication failed'),
        reason: 'ApiAuthException should produce a clear auth error message',
      );
      expect(state.conversations, isEmpty);
    });

    // -----------------------------------------------------------------------
    // REGRESSION: race condition — null → non-null rebuild
    // -----------------------------------------------------------------------

    test(
      'REGRESSION: rebuilds and subscribes when apiClientProvider transitions '
      'from null (context loading) to non-null (context ready)',
      () async {
        final container = ProviderContainer(
          overrides: [
            apiClientProvider.overrideWith(
              (ref) => ref.watch(_apiClientHolderProvider),
            ),
          ],
        );
        addTearDown(container.dispose);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        // Phase 1: no client — empty state.
        await container.read(conversationListProvider.future);
        expect(
          container.read(conversationListProvider).value?.conversations,
          isEmpty,
          reason: 'no conversations expected while apiClientProvider is null',
        );

        // Phase 2: client becomes available — notifier should rebuild and subscribe.
        final controller = StreamController<ConversationListEvent>.broadcast();
        addTearDown(controller.close);
        final client = _FakeApiClient(streamController: controller);
        container.read(_apiClientHolderProvider.notifier).set(client);
        await Future<void>.delayed(Duration.zero);
        await container.read(conversationListProvider.future);

        // Emit snapshot via the stream.
        controller.add(ConversationSnapshotEvent([_makeEntry(id: 'c-loaded')]));
        await Future<void>.delayed(Duration.zero);

        expect(
          container
              .read(conversationListProvider)
              .value
              ?.conversations
              .map((c) => c.id),
          contains('c-loaded'),
          reason:
              'conversations must be loaded after apiClientProvider becomes '
              'non-null; this regresses if ref.watch() is replaced with '
              'ref.read() in build()',
        );
      },
    );

    // -----------------------------------------------------------------------
    // refresh()
    // -----------------------------------------------------------------------

    test('refresh() reconnects the stream', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(ConversationSnapshotEvent([_makeEntry(id: 'c1')]));
      await Future<void>.delayed(Duration.zero);
      expect(
        container.read(conversationListProvider).value?.conversations.length,
        1,
      );

      // Refresh reconnects — emit a new snapshot with updated data
      await container.read(conversationListProvider.notifier).refresh();
      controller.add(
        ConversationSnapshotEvent([_makeEntry(id: 'c1'), _makeEntry(id: 'c2')]),
      );
      await Future<void>.delayed(Duration.zero);

      final after = container.read(conversationListProvider).value!;
      expect(after.conversations.length, 2);
      expect(after.error, isNull);
    });

    test('refresh() with null apiClient is a no-op', () async {
      final container = _makeContainer();
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      await container.read(conversationListProvider.notifier).refresh();

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations, isEmpty);
      expect(state.error, isNull);
    });

    // -----------------------------------------------------------------------
    // prependConversation()
    // -----------------------------------------------------------------------

    test('prependConversation inserts at the front of the list', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(id: 'old', updatedAt: DateTime(2024, 1, 1)),
        ]),
      );
      await Future<void>.delayed(Duration.zero);

      final newConv = _makeConv(id: 'new', title: 'Brand new');
      container
          .read(conversationListProvider.notifier)
          .prependConversation(newConv);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 2);
      expect(
        state.conversations.first.id,
        'new',
        reason: 'new conversation must appear at the top',
      );
    });

    test('prependConversation works when list is initially empty', () async {
      final container = _makeContainer();
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      final conv = _makeConv(id: 'first');
      container
          .read(conversationListProvider.notifier)
          .prependConversation(conv);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 1);
      expect(state.conversations.first.id, 'first');
    });

    // -----------------------------------------------------------------------
    // Debounce
    // -----------------------------------------------------------------------

    test(
      'debounces rapid upserted events into a single state rebuild',
      () async {
        final controller = StreamController<ConversationListEvent>.broadcast();
        final client = _FakeApiClient(streamController: controller);
        final container = _makeContainer(client: client);
        addTearDown(container.dispose);
        addTearDown(controller.close);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        await container.read(conversationListProvider.future);

        // Emit initial snapshot
        controller.add(ConversationSnapshotEvent([_makeEntry(id: 'c1')]));
        await Future<void>.delayed(Duration.zero);

        // Track state rebuilds after snapshot
        int rebuildCount = 0;
        container.listen(conversationListProvider, (_, _) {
          rebuildCount++;
        });

        // Emit 5 rapid upserts for the same conversation
        for (int i = 0; i < 5; i++) {
          controller.add(
            ConversationUpsertedEvent(
              _makeEntry(
                id: 'c1',
                title: 'Update $i',
                updatedAt: DateTime(2024, 1, 1, 0, 0, i),
              ),
            ),
          );
        }

        // Wait for events to be delivered and debounce timer to fire
        await Future<void>.delayed(const Duration(milliseconds: 500));

        // Only one rebuild should have occurred (debounced batch)
        expect(
          rebuildCount,
          1,
          reason: 'rapid upserts should be debounced into one rebuild',
        );

        // Final state should have the latest title
        final state = container.read(conversationListProvider).value!;
        expect(state.conversations.first.title, 'Update 4');
      },
    );

    // -----------------------------------------------------------------------
    // createConversation()
    // -----------------------------------------------------------------------

    test(
      'createConversation sends no title so the server auto-titles the chat',
      () async {
        final controller = StreamController<ConversationListEvent>.broadcast();
        final fakeConversationsApi = _FakeConversationsApi();
        final client = _FakeApiClient(
          streamController: controller,
          conversationsApi: fakeConversationsApi,
        );
        final container = _makeContainer(client: client);
        addTearDown(container.dispose);
        addTearDown(controller.close);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        await container.read(conversationListProvider.future);

        final id = await container
            .read(conversationListProvider.notifier)
            .createConversation();

        expect(id, 'created-1');
        expect(fakeConversationsApi.createRequests, hasLength(1));
        expect(
          fakeConversationsApi.createRequests.single.title,
          isNull,
          reason: 'An explicit title locks the row and disables auto-titling',
        );
      },
    );

    // -----------------------------------------------------------------------
    // deleteConversation()
    // -----------------------------------------------------------------------

    test(
      'deleteConversation calls API and stream delivers the removal',
      () async {
        final controller = StreamController<ConversationListEvent>.broadcast();
        final fakeConversationsApi = _FakeConversationsApi();
        final client = _FakeApiClient(
          streamController: controller,
          conversationsApi: fakeConversationsApi,
        );
        final container = _makeContainer(client: client);
        addTearDown(container.dispose);
        addTearDown(controller.close);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        await container.read(conversationListProvider.future);

        controller.add(
          ConversationSnapshotEvent([
            _makeEntry(id: 'keep'),
            _makeEntry(id: 'remove'),
          ]),
        );
        await Future<void>.delayed(Duration.zero);
        expect(
          container.read(conversationListProvider).value?.conversations.length,
          2,
        );

        // Delete via notifier — fires API call
        await container
            .read(conversationListProvider.notifier)
            .deleteConversation('remove');
        expect(fakeConversationsApi.deletedIds, contains('remove'));

        // Stream delivers the removal
        controller.add(const ConversationDeletedEvent('remove'));
        await Future<void>.delayed(Duration.zero);

        final state = container.read(conversationListProvider).value!;
        expect(state.conversations.length, 1);
        expect(state.conversations.first.id, 'keep');
        expect(state.conversations.any((c) => c.id == 'remove'), isFalse);
      },
    );

    // -------------------------------------------------------------------
    // Auto-reconnect on stream error
    // -------------------------------------------------------------------

    test('auto-reconnects on non-auth stream error', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);
      final initialCallCount = client.streamConversationsCallCount;

      // Emit an error on the stream.
      controller.addError(Exception('connection lost'));
      // Wait for reconnect delay (2s base) + buffer.
      await Future<void>.delayed(const Duration(seconds: 3));

      expect(
        client.streamConversationsCallCount,
        greaterThan(initialCallCount),
        reason: 'should attempt to reconnect after non-auth error',
      );
    });

    test('does not auto-reconnect on ApiAuthException', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);
      final initialCallCount = client.streamConversationsCallCount;

      // Emit an auth error.
      controller.addError(const ApiAuthException());
      await Future<void>.delayed(const Duration(seconds: 3));

      expect(
        client.streamConversationsCallCount,
        equals(initialCallCount),
        reason: 'should NOT reconnect on auth errors',
      );
      final state = container.read(conversationListProvider).value!;
      expect(state.error, isNotNull);
    });

    test('gives up after max reconnect attempts', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      // Emit errors to exhaust reconnect attempts.
      // Each error triggers a reconnect with increasing delay.
      // Delays: 2s, 4s, 8s, 16s, 30s = 60s total — we'll speed this up
      // by emitting errors rapidly (each reconnect re-subscribes to the
      // same broadcast stream, so the next error hits immediately).
      for (var i = 0; i < 6; i++) {
        controller.addError(Exception('error $i'));
        await Future<void>.delayed(const Duration(seconds: 3));
      }

      final state = container.read(conversationListProvider).value!;
      expect(
        state.error,
        isNotNull,
        reason: 'should show error after all reconnect attempts exhausted',
      );
    });

    test('resets reconnect attempts on successful snapshot', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      // Trigger an error and reconnect.
      controller.addError(Exception('blip'));
      await Future<void>.delayed(const Duration(seconds: 3));

      // Reconnected — send a snapshot to reset attempt counter.
      controller.add(
        ConversationSnapshotEvent([_makeEntry(id: 'c1', title: 'Back')]),
      );
      await Future<void>.delayed(Duration.zero);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.length, 1);
      expect(state.error, isNull);

      // Another error should trigger reconnect (attempt counter was reset).
      final callCountBefore = client.streamConversationsCallCount;
      controller.addError(Exception('another blip'));
      await Future<void>.delayed(const Duration(seconds: 3));

      expect(
        client.streamConversationsCallCount,
        greaterThan(callCountBefore),
        reason: 'should reconnect again after counter was reset by snapshot',
      );
    });

    test('preserves existing conversations during reconnect', () async {
      final controller = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(streamController: controller);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(controller.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      // Load initial data.
      controller.add(
        ConversationSnapshotEvent([
          _makeEntry(id: 'c1', title: 'Keep'),
          _makeEntry(id: 'c2', title: 'Also Keep'),
        ]),
      );
      await Future<void>.delayed(Duration.zero);

      // Error on the stream.
      controller.addError(Exception('network blip'));
      await Future<void>.delayed(const Duration(seconds: 1));

      // Conversations should still be visible during reconnect.
      final state = container.read(conversationListProvider).value!;
      expect(
        state.conversations.length,
        2,
        reason: 'existing conversations should be preserved during reconnect',
      );
    });
  });

  // -------------------------------------------------------------------------
  // Space-scoping: rebuild on space change
  // -------------------------------------------------------------------------

  group('metadata actions', () {
    Future<
      ({
        ProviderContainer container,
        _FakeConversationsApi api,
        StreamController<ConversationListEvent> stream,
      })
    >
    setUp() async {
      final api = _FakeConversationsApi();
      final stream = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(
        streamController: stream,
        conversationsApi: api,
      );
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(stream.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);
      await container.read(conversationListProvider.future);
      stream.add(
        ConversationSnapshotEvent([_makeEntry(id: 'c1', title: 'Original')]),
      );
      await Future<void>.delayed(Duration.zero);
      return (container: container, api: api, stream: stream);
    }

    test('renameConversation PATCHes only the title', () async {
      final t = await setUp();
      final error = await t.container
          .read(conversationListProvider.notifier)
          .renameConversation('c1', 'Renamed');

      expect(error, isNull);
      expect(t.api.updates, hasLength(1));
      expect(t.api.updates.single.$1, 'c1');
      expect(t.api.updates.single.$2.title, 'Renamed');
      expect(t.api.updates.single.$2.pinned, isNull);
    });

    test('setPinned PATCHes only the pinned flag', () async {
      final t = await setUp();
      final notifier = t.container.read(conversationListProvider.notifier);

      expect(await notifier.setPinned('c1', true), isNull);
      expect(await notifier.setPinned('c1', false), isNull);

      expect(t.api.updates.map((u) => u.$2.pinned), [true, false]);
      expect(t.api.updates.every((u) => u.$2.title == null), isTrue);
    });

    test('does not change the list optimistically', () async {
      final t = await setUp();
      await t.container
          .read(conversationListProvider.notifier)
          .renameConversation('c1', 'Renamed');
      await _waitForDebounce();

      final list = t.container.read(conversationListProvider).value!;
      expect(list.conversations.single.title, 'Original');
    });

    test('the stream delivers the renamed title', () async {
      final t = await setUp();
      await t.container
          .read(conversationListProvider.notifier)
          .renameConversation('c1', 'Renamed');
      t.stream.add(
        ConversationUpsertedEvent(_makeEntry(id: 'c1', title: 'Renamed')),
      );
      await _waitForDebounce();

      final list = t.container.read(conversationListProvider).value!;
      expect(list.conversations.single.title, 'Renamed');
    });

    test('renameConversation returns the error and leaves state', () async {
      final t = await setUp();
      t.api.updateError = DioException(
        requestOptions: RequestOptions(path: '/api/conversations/c1'),
        response: Response(
          requestOptions: RequestOptions(path: '/api/conversations/c1'),
          statusCode: 400,
          data: {'error': 'title too long'},
        ),
        message: 'Bad status',
      );
      final before = t.container.read(conversationListProvider).value!;

      final error = await t.container
          .read(conversationListProvider.notifier)
          .renameConversation('c1', 'Renamed');

      expect(error, 'title too long');
      final after = t.container.read(conversationListProvider).value!;
      expect(after.conversations, before.conversations);
      expect(after.error, isNull, reason: 'list error banner is untouched');
    });

    test('setPinned returns a generic error for non-API failures', () async {
      final t = await setUp();
      t.api.updateError = StateError('boom');

      final error = await t.container
          .read(conversationListProvider.notifier)
          .setPinned('c1', true);

      expect(error, isNotNull);
      expect(error, isNotEmpty);
    });

    test('returns an error when there is no API client', () async {
      final container = _makeContainer();
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);
      await container.read(conversationListProvider.future);

      final error = await container
          .read(conversationListProvider.notifier)
          .setPinned('c1', true);

      expect(error, isNotNull);
    });
  });

  group('archive', () {
    Future<
      ({
        ProviderContainer container,
        _FakeConversationsApi api,
        StreamController<ConversationListEvent> stream,
        ConversationListNotifier notifier,
      })
    >
    setUp({List<ConversationListEntry>? snapshot}) async {
      final api = _FakeConversationsApi();
      final stream = StreamController<ConversationListEvent>.broadcast();
      final client = _FakeApiClient(
        streamController: stream,
        conversationsApi: api,
      );
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);
      addTearDown(stream.close);
      final sub = _keepAlive(container);
      addTearDown(sub.close);
      await container.read(conversationListProvider.future);
      stream.add(
        ConversationSnapshotEvent(
          snapshot ??
              [
                _makeEntry(id: 'c1', updatedAt: DateTime(2024, 3, 1)),
                _makeEntry(id: 'c2', updatedAt: DateTime(2024, 2, 1)),
              ],
        ),
      );
      await Future<void>.delayed(Duration.zero);
      return (
        container: container,
        api: api,
        stream: stream,
        notifier: container.read(conversationListProvider.notifier),
      );
    }

    ConversationListState read(ProviderContainer c) =>
        c.read(conversationListProvider).value!;

    List<String> archivedIds(ProviderContainer c) =>
        read(c).archived.conversations.map((e) => e.id).toList();

    List<String> mainIds(ProviderContainer c) =>
        read(c).conversations.map((e) => e.id).toList();

    test('setArchived PATCHes only the archived flag', () async {
      final t = await setUp();

      expect(await t.notifier.setArchived('c1', true), isNull);
      expect(await t.notifier.setArchived('c1', false), isNull);

      expect(t.api.updates.map((u) => u.$2.archived), [true, false]);
      expect(t.api.updates.every((u) => u.$2.title == null), isTrue);
      expect(t.api.updates.every((u) => u.$2.pinned == null), isTrue);
    });

    test('setArchived returns the error and leaves state', () async {
      final t = await setUp();
      t.api.updateError = StateError('boom');

      final error = await t.notifier.setArchived('c1', true);

      expect(error, isNotNull);
      expect(mainIds(t.container), ['c1', 'c2']);
    });

    test('setPinned on an archived entry sends only pinned and the upsert '
        'moves it to the Pinned group', () async {
      final t = await setUp();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.setArchivedExpanded(true);
      expect(archivedIds(t.container), ['a1']);

      expect(await t.notifier.setPinned('a1', true), isNull);
      expect(t.api.updates.single.$2.pinned, isTrue);
      expect(t.api.updates.single.$2.archived, isNull);

      t.stream.add(
        ConversationUpsertedEvent(
          _makeEntry(id: 'a1', pinnedAt: DateTime(2024, 6, 1)),
        ),
      );
      await _waitForDebounce();

      expect(archivedIds(t.container), isEmpty);
      expect(mainIds(t.container), ['a1', 'c1', 'c2']);
      expect(read(t.container).conversations.first.pinnedAt, isNotNull);
    });

    test('an upsert with archived_at leaves the main list and joins the '
        'loaded archived list', () async {
      final t = await setUp();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.setArchivedExpanded(true);

      t.stream.add(
        ConversationUpsertedEvent(
          _makeEntry(
            id: 'c1',
            updatedAt: DateTime(2024, 3, 1),
            archivedAt: DateTime(2024, 6, 1),
          ),
        ),
      );
      await _waitForDebounce();

      expect(mainIds(t.container), ['c2']);
      expect(archivedIds(t.container), ['c1', 'a1'], reason: 'newest first');
    });

    test('an upsert with archived_at before the section loads only removes '
        'the row from the main list', () async {
      final t = await setUp();

      t.stream.add(
        ConversationUpsertedEvent(
          _makeEntry(id: 'c1', archivedAt: DateTime(2024, 6, 1)),
        ),
      );
      await _waitForDebounce();

      expect(mainIds(t.container), ['c2']);
      expect(read(t.container).archived.conversations, isEmpty);
      expect(t.api.listCalls, isEmpty, reason: 'no fetch until expanded');
    });

    test('an upsert without archived_at moves an archived row back in '
        'server order', () async {
      final t = await setUp();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.setArchivedExpanded(true);

      t.stream.add(
        ConversationUpsertedEvent(
          _makeEntry(id: 'a1', updatedAt: DateTime(2024, 2, 15)),
        ),
      );
      await _waitForDebounce();

      expect(archivedIds(t.container), isEmpty);
      expect(mainIds(t.container), ['c1', 'a1', 'c2']);
    });

    test('a deleted event removes the entry from the archived list', () async {
      final t = await setUp();
      t.api.listResponse = [
        _makeArchivedSummary('a1', DateTime(2024, 5, 1)),
        _makeArchivedSummary('a2', DateTime(2024, 4, 1)),
      ];
      await t.notifier.setArchivedExpanded(true);

      t.stream.add(const ConversationDeletedEvent('a1'));
      await Future<void>.delayed(Duration.zero);

      expect(archivedIds(t.container), ['a2']);
    });

    test('a snapshot while expanded refetches and replaces the archived '
        'list', () async {
      final t = await setUp();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.setArchivedExpanded(true);
      expect(t.api.listCalls, [true]);

      t.api.listResponse = [_makeArchivedSummary('a2', DateTime(2024, 6, 1))];
      t.stream.add(ConversationSnapshotEvent([_makeEntry(id: 'c1')]));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(t.api.listCalls, [true, true]);
      expect(archivedIds(t.container), ['a2']);
      expect(read(t.container).archived.expanded, isTrue);
      expect(mainIds(t.container), ['c1']);
    });

    test('a snapshot while collapsed does not fetch', () async {
      final t = await setUp();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.setArchivedExpanded(true);
      await t.notifier.setArchivedExpanded(false);
      t.api.listCalls.clear();

      t.stream.add(ConversationSnapshotEvent([_makeEntry(id: 'c1')]));
      await Future<void>.delayed(Duration.zero);

      expect(t.api.listCalls, isEmpty);
      expect(read(t.container).archived.expanded, isFalse);
    });

    test('loadArchived fetches only on demand, with archived=true', () async {
      final t = await setUp();
      expect(t.api.listCalls, isEmpty);
      expect(read(t.container).archived.loaded, isFalse);

      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.loadArchived();

      expect(t.api.listCalls, [true]);
      expect(read(t.container).archived.loaded, isTrue);
      expect(archivedIds(t.container), ['a1']);
      expect(read(t.container).archived.error, isNull);
    });

    test('loadArchived reports loading, then success', () async {
      final t = await setUp();
      t.api.listGate = Completer<void>();
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];

      final pending = t.notifier.loadArchived();
      await Future<void>.delayed(Duration.zero);
      expect(read(t.container).archived.loading, isTrue);

      t.api.listGate!.complete();
      await pending;
      expect(read(t.container).archived.loading, isFalse);
      expect(archivedIds(t.container), ['a1']);
    });

    test('loadArchived reports an error and keeps the main list', () async {
      final t = await setUp();
      t.api.listError = StateError('boom');

      await t.notifier.loadArchived();

      final archived = read(t.container).archived;
      expect(archived.loading, isFalse);
      expect(archived.error, isNotNull);
      expect(archived.loaded, isFalse);
      expect(mainIds(t.container), ['c1', 'c2']);
      expect(read(t.container).error, isNull);
    });

    test('expanding again after an error retries', () async {
      final t = await setUp();
      t.api.listError = StateError('boom');
      await t.notifier.setArchivedExpanded(true);
      expect(read(t.container).archived.error, isNotNull);

      t.api.listError = null;
      t.api.listResponse = [_makeArchivedSummary('a1', DateTime(2024, 5, 1))];
      await t.notifier.loadArchived();

      expect(read(t.container).archived.error, isNull);
      expect(archivedIds(t.container), ['a1']);
    });
  });

  group('space-scoping', () {
    test('rebuilds (re-subscribes) when space selection changes', () async {
      final client = _FakeApiClient();
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(client)],
      );
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      // Initial subscription.
      await Future<void>.delayed(Duration.zero);
      expect(client.streamConversationsCallCount, 1);

      // Change space selection — should trigger a rebuild.
      container
          .read(spaceSelectionProvider.notifier)
          .selectOrg(orgId: 'org-1', orgName: 'Org 1');
      await Future<void>.delayed(Duration.zero);

      // A new subscription means streamConversations was called again.
      expect(
        client.streamConversationsCallCount,
        greaterThan(1),
        reason: 'space change should trigger stream re-subscription',
      );
    });
  });
}
