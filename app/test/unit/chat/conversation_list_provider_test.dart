import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fake ConversationsApi — no HTTP, returns controlled data.

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi({this.conversations = const [], this.error})
    : super(Dio(), standardSerializers);

  final List<ConversationSummary> conversations;
  final Exception? error;

  @override
  Future<Response<BuiltList<ConversationSummary>>> listConversations({
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    if (error != null) throw error!;
    return Response(
      data: BuiltList.of(conversations),
      requestOptions: RequestOptions(path: '/api/conversations'),
      statusCode: 200,
    );
  }

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
    conversations.removeWhere((c) => c.id == id);
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 204,
    );
  }
}

// Wraps _FakeConversationsApi in an ApiClient-compatible surface.
class _FakeApiClient extends ApiClient {
  _FakeApiClient({required this.fakeApi})
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeConversationsApi fakeApi;

  @override
  ConversationsApi get conversations => fakeApi;
}

// ---------------------------------------------------------------------------
// Test helpers

/// Build a minimal [ConversationSummary] for testing.
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
// StateProvider was removed in Riverpod 3.x — use Notifier instead.

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
    // Happy path
    // -----------------------------------------------------------------------

    test(
      'loads conversations when apiClientProvider is available from the start',
      () async {
        final fakeApi = _FakeConversationsApi(
          conversations: [
            _makeConv(id: 'c1', title: 'First'),
            _makeConv(id: 'c2', title: 'Second'),
          ],
        );
        final container = _makeContainer(
          client: _FakeApiClient(fakeApi: fakeApi),
        );
        addTearDown(container.dispose);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        final state = await container.read(conversationListProvider.future);
        expect(state.conversations.length, 2);
        expect(state.conversations.map((c) => c.id), containsAll(['c1', 'c2']));
        expect(state.error, isNull);
      },
    );

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    test('captures error when listConversations throws', () async {
      final fakeApi = _FakeConversationsApi(
        error: Exception('network failure'),
      );
      final container = _makeContainer(
        client: _FakeApiClient(fakeApi: fakeApi),
      );
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      final state = await container.read(conversationListProvider.future);
      expect(state.error, isNotNull);
      expect(state.error, contains('network failure'));
      expect(state.conversations, isEmpty);
    });

    // -----------------------------------------------------------------------
    // REGRESSION: race condition — null → non-null rebuild
    // -----------------------------------------------------------------------

    test('REGRESSION: rebuilds and fetches when apiClientProvider transitions '
        'from null (context loading) to non-null (context ready)', () async {
      // Simulates the race condition that occurred on web:
      // apiClientProvider starts null because the active context is still
      // loading asynchronously. The fix (ref.watch instead of ref.read in
      // build()) causes the notifier to rebuild when the client appears.
      final container = ProviderContainer(
        overrides: [
          apiClientProvider.overrideWith(
            (ref) => ref.watch(_apiClientHolderProvider),
          ),
        ],
      );
      addTearDown(container.dispose);

      // Keep the autoDispose provider alive across the state transition.
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      // Phase 1: no client — must return empty state, no error.
      await container.read(conversationListProvider.future);
      expect(
        container.read(conversationListProvider).value?.conversations,
        isEmpty,
        reason: 'no conversations expected while apiClientProvider is null',
      );

      // Phase 2: active context finishes loading → apiClient becomes available.
      final fakeApi = _FakeConversationsApi(
        conversations: [_makeConv(id: 'c-loaded')],
      );
      container
          .read(_apiClientHolderProvider.notifier)
          .set(_FakeApiClient(fakeApi: fakeApi));

      // Yield so Riverpod can propagate the dependency invalidation.
      await Future<void>.delayed(Duration.zero);

      // Await the reactive rebuild triggered by the apiClient change.
      await container.read(conversationListProvider.future);

      expect(
        container
            .read(conversationListProvider)
            .value
            ?.conversations
            .map((c) => c.id),
        contains('c-loaded'),
        reason:
            'conversations must be loaded after apiClientProvider becomes non-null; '
            'this regresses if ref.watch() is replaced with ref.read() in build()',
      );
    });

    // -----------------------------------------------------------------------
    // refresh()
    // -----------------------------------------------------------------------

    test('refresh() reloads conversations from the server', () async {
      final fakeApi = _FakeConversationsApi(
        conversations: [_makeConv(id: 'c1')],
      );
      final container = _makeContainer(
        client: _FakeApiClient(fakeApi: fakeApi),
      );
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);
      expect(
        container.read(conversationListProvider).value?.conversations.length,
        1,
      );

      await container.read(conversationListProvider.notifier).refresh();

      final after = container.read(conversationListProvider).value!;
      expect(after.conversations.length, 1);
      expect(after.conversations.first.id, 'c1');
      expect(after.error, isNull);
    });

    test('refresh() with null apiClient is a no-op', () async {
      final container = _makeContainer();
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      // Should not throw.
      await container.read(conversationListProvider.notifier).refresh();

      final state = container.read(conversationListProvider).value!;
      expect(state.conversations, isEmpty);
      expect(state.error, isNull);
    });

    // -----------------------------------------------------------------------
    // prependConversation()
    // -----------------------------------------------------------------------

    test('prependConversation inserts at the front of the list', () async {
      final fakeApi = _FakeConversationsApi(
        conversations: [_makeConv(id: 'old')],
      );
      final container = _makeContainer(
        client: _FakeApiClient(fakeApi: fakeApi),
      );
      addTearDown(container.dispose);
      final sub = _keepAlive(container);
      addTearDown(sub.close);

      await container.read(conversationListProvider.future);

      final newConv = _makeConv(id: 'new', title: 'Brand new');
      container
          .read(conversationListProvider.notifier)
          .prependConversation(newConv);

      final state = container.read(conversationListProvider).value!;
      expect(
        state.conversations.first.id,
        'new',
        reason: 'new conversation must appear at the top',
      );
      expect(state.conversations.length, 2);
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
      'deleteConversation removes the conversation from local state',
      () async {
        final fakeApi = _FakeConversationsApi(
          conversations: [
            _makeConv(id: 'keep'),
            _makeConv(id: 'remove'),
          ],
        );
        final container = _makeContainer(
          client: _FakeApiClient(fakeApi: fakeApi),
        );
        addTearDown(container.dispose);
        final sub = _keepAlive(container);
        addTearDown(sub.close);

        await container.read(conversationListProvider.future);
        expect(
          container.read(conversationListProvider).value?.conversations.length,
          2,
        );

        await container
            .read(conversationListProvider.notifier)
            .deleteConversation('remove');

        final state = container.read(conversationListProvider).value!;
        expect(state.conversations.length, 1);
        expect(state.conversations.first.id, 'keep');
        expect(state.conversations.any((c) => c.id == 'remove'), isFalse);
      },
    );
  });
}
