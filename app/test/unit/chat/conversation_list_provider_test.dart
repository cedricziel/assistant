import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
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

  @override
  Stream<ConversationListEvent> streamConversations({String? agentId}) {
    return streamController.stream;
  }

  @override
  ConversationsApi get conversations => _conversationsApi;
}

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  final List<String> deletedIds = [];

  @override
  Future<Response<void>> deleteConversation({
    required String id,
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
}) {
  return ConversationListEntry(
    id: id,
    title: title,
    createdAt: DateTime(2024, 1, 1),
    updatedAt: updatedAt ?? DateTime(2024, 1, 1),
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
      await Future<void>.delayed(Duration.zero);

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
      await Future<void>.delayed(Duration.zero);

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations.first.id, 'c2');
      expect(state.conversations.first.title, 'Updated');
    });

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    test('captures error when stream emits an error', () async {
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

      final state = container.read(conversationListProvider).value!;
      expect(state.error, isNotNull);
      expect(state.error, contains('network failure'));
      expect(state.conversations, isEmpty);
    });

    // -----------------------------------------------------------------------
    // REGRESSION: race condition — null → non-null rebuild
    // -----------------------------------------------------------------------

    test('REGRESSION: rebuilds and subscribes when apiClientProvider transitions '
        'from null (context loading) to non-null (context ready)', () async {
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
    });

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
  });
}
