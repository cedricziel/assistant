import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide BranchInfo;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  /// Message ids sent to switch-branch, in call order.
  final List<String> switched = [];

  /// Gates the reply while set.
  Completer<void>? gate;

  /// Status code to fail with; null answers with [detail].
  int? failWith;

  /// Fails with a connection error (no response) while set.
  bool offline = false;

  ConversationDetail detail = _detail([('u1', 'user', 'hello')]);

  @override
  Future<Response<ConversationDetail>> switchConversationBranch({
    required String id,
    required SwitchBranchRequest switchBranchRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    switched.add(switchBranchRequest.messageId);
    await gate?.future;
    final path = RequestOptions(path: '/api/conversations/$id/switch-branch');
    if (offline) {
      throw DioException(
        requestOptions: path,
        type: DioExceptionType.connectionError,
      );
    }
    final code = failWith;
    if (code != null) {
      throw DioException(
        requestOptions: path,
        type: DioExceptionType.badResponse,
        response: Response<void>(requestOptions: path, statusCode: code),
      );
    }
    return Response(requestOptions: path, statusCode: 200, data: detail);
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient() : super(baseUrl: 'http://fake', token: 'fake');

  final _FakeConversationsApi fake = _FakeConversationsApi();

  @override
  ConversationsApi get conversations => fake;
}

ConversationDetail _detail(List<(String, String, String)> rows) =>
    ConversationDetail(
      (b) => b
        ..id = 'conv-1'
        ..title = 'T'
        ..createdAt = DateTime(2024)
        ..updatedAt = DateTime(2024)
        ..messages.replace([
          for (final (id, role, content) in rows)
            MessageSummary(
              (m) => m
                ..id = id
                ..role = role
                ..content = content
                ..createdAt = DateTime(2024)
                ..turn = 0
                ..ttsAvailable = false,
            ),
        ]),
    );

({ProviderContainer container, _FakeApiClient api}) _setup() {
  final api = _FakeApiClient();
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(api),
      conversationListProvider.overrideWith(_FakeListNotifier.new),
    ],
  );
  addTearDown(container.dispose);
  return (container: container, api: api);
}

class _FakeListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();
}

Future<ChatNotifier> _notifier(ProviderContainer container) async {
  container.listen(chatProvider, (_, _) {});
  await container.read(chatProvider.future);
  final notifier = container.read(chatProvider.notifier)
    ..setConversationId('conv-1');
  return notifier;
}

void main() {
  group('ChatNotifier.switchBranch', () {
    test('marks the picker busy while the request is pending', () async {
      final s = _setup();
      final notifier = await _notifier(s.container);
      s.api.fake.gate = Completer<void>();

      final pending = notifier.switchBranch('u1-b');
      await Future<void>.delayed(Duration.zero);

      expect(s.container.read(chatProvider).value!.isSwitchingBranch, isTrue);
      expect(s.api.fake.switched, ['u1-b']);

      s.api.fake.gate!.complete();
      await pending;
      expect(s.container.read(chatProvider).value!.isSwitchingBranch, isFalse);
    });

    test('replaces the messages with the returned detail', () async {
      final s = _setup();
      final notifier = await _notifier(s.container);
      s.api.fake.detail = _detail([
        ('u1-b', 'user', 'edited'),
        ('a1-b', 'assistant', 'new reply'),
      ]);

      await notifier.switchBranch('u1-b');

      final state = s.container.read(chatProvider).value!;
      expect(state.messages.map((m) => m.id), ['u1-b', 'a1-b']);
      expect(state.conversationId, 'conv-1');
      expect(state.error, isNull);
    });

    for (final status in [409, 500]) {
      test(
        'a $status leaves the messages unchanged and sets an error',
        () async {
          final s = _setup();
          final notifier = await _notifier(s.container);
          s.api.fake.detail = _detail([('u1', 'user', 'hello')]);
          await notifier.switchBranch('u1');
          final before = s.container.read(chatProvider).value!.messages;
          s.api.fake.failWith = status;

          await notifier.switchBranch('u1-b');

          final state = s.container.read(chatProvider).value!;
          expect(state.messages, same(before));
          expect(state.error, isNotNull);
          expect(state.isSwitchingBranch, isFalse);
        },
      );
    }

    test('a network error leaves the messages unchanged', () async {
      final s = _setup();
      final notifier = await _notifier(s.container);
      await notifier.switchBranch('u1');
      final before = s.container.read(chatProvider).value!.messages;
      s.api.fake.offline = true;

      await notifier.switchBranch('u1-b');

      final state = s.container.read(chatProvider).value!;
      expect(state.messages, same(before));
      expect(state.error, isNotNull);
      expect(state.isSwitchingBranch, isFalse);
    });
  });
}
