import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/message_feedback_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fakes -------------------------------------------------------------------

/// One recorded feedback call: `rating == null` is a DELETE.
typedef _Call = ({
  String conversationId,
  String messageId,
  String? rating,
  String? comment,
});

/// A feedback call the test has to settle by hand.
class _Held {
  _Held(this.call);
  final _Call call;
  final Completer<void> _done = Completer<void>();

  void succeed() => _done.complete();
  void fail() => _done.completeError(
    DioException(
      requestOptions: RequestOptions(path: '/feedback'),
      type: DioExceptionType.badResponse,
    ),
  );
}

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  /// Feedback calls in the order they reached the API.
  final List<_Call> calls = [];

  /// When true, calls wait for the test to settle them through [held].
  bool hold = false;
  final List<_Held> held = [];

  /// Persisted feedback per conversation returned by `getConversation`.
  final Map<String, List<({String id, String? rating, String? comment})>>
  history = {};

  Future<void> _record(_Call call) async {
    calls.add(call);
    if (!hold) return;
    final h = _Held(call);
    held.add(h);
    await h._done.future;
  }

  @override
  Future<Response<MessageFeedbackResponse>> setMessageFeedback({
    required String id,
    required String messageId,
    required SetMessageFeedbackRequest setMessageFeedbackRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    await _record((
      conversationId: id,
      messageId: messageId,
      rating: setMessageFeedbackRequest.rating.name,
      comment: setMessageFeedbackRequest.comment,
    ));
    return Response(
      requestOptions: RequestOptions(path: '/feedback'),
      statusCode: 200,
      data: MessageFeedbackResponse(
        (b) => b
          ..messageId = messageId
          ..rating = setMessageFeedbackRequest.rating
          ..comment = setMessageFeedbackRequest.comment
          ..createdAt = DateTime(2024)
          ..updatedAt = DateTime(2024),
      ),
    );
  }

  @override
  Future<Response<void>> deleteMessageFeedback({
    required String id,
    required String messageId,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    await _record((
      conversationId: id,
      messageId: messageId,
      rating: null,
      comment: null,
    ));
    return Response(
      requestOptions: RequestOptions(path: '/feedback'),
      statusCode: 204,
    );
  }

  @override
  Future<Response<ConversationDetail>> getConversation({
    required String id,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    final rows = history[id] ?? const [];
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 200,
      data: ConversationDetail(
        (b) => b
          ..id = id
          ..title = 'T'
          ..createdAt = DateTime(2024)
          ..updatedAt = DateTime(2024)
          ..messages.replace([
            for (final r in rows)
              MessageSummary(
                (m) => m
                  ..id = r.id
                  ..role = 'assistant'
                  ..content = 'answer ${r.id}'
                  ..createdAt = DateTime(2024)
                  ..turn = 0
                  ..ttsAvailable = false
                  ..feedback = r.rating == null
                      ? null
                      : (MessageFeedbackSummaryBuilder()
                          ..rating = r.rating!
                          ..comment = r.comment
                          ..updatedAt = DateTime(2024)),
              ),
          ]),
      ),
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient() : super(baseUrl: 'http://fake', token: 'fake');

  final _FakeConversationsApi fake = _FakeConversationsApi();

  @override
  ConversationsApi get conversations => fake;

  @override
  Future<List<InteractionDetail>> listConversationInteractions(
    String conversationId, {
    InteractionStatusView? status,
  }) async => const [];
}

// -- Harness -----------------------------------------------------------------

class _Harness {
  _Harness(this.container, this.api);

  final ProviderContainer container;
  final _FakeApiClient api;

  ChatNotifier get chat => container.read(chatProvider.notifier);
  ChatState get state => container.read(chatProvider).requireValue;
  MessageFeedbackController get feedback =>
      container.read(messageFeedbackControllerProvider.notifier);

  MessageFeedback? feedbackOf(String messageId) =>
      state.messages.firstWhere((m) => m.id == messageId).feedback;

  /// Let queued microtasks and futures run.
  Future<void> settle() => pumpEventQueue();
}

/// Builds a harness with conversation `conv-1` loaded from [history].
Future<_Harness> _harness({
  List<({String id, String? rating, String? comment})> history = const [],
}) async {
  final api = _FakeApiClient();
  api.fake.history['conv-1'] = history;
  final container = ProviderContainer(
    overrides: [apiClientProvider.overrideWithValue(api)],
  );
  addTearDown(container.dispose);
  // Keep the autoDispose chat provider alive for the test.
  container.listen(chatProvider, (_, _) {});
  await container.read(chatProvider.future);
  await container.read(chatProvider.notifier).loadConversation('conv-1');
  return _Harness(container, api);
}

/// One assistant message, `m1`, without a vote.
const List<({String id, String? rating, String? comment})> _unrated = [
  (id: 'm1', rating: null, comment: null),
];

const _positive = MessageFeedback(rating: MessageRating.positive);
const _negative = MessageFeedback(rating: MessageRating.negative);

void main() {
  group('optimistic set', () {
    test('applies immediately, PUTs, and settles the pending flag', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'negative', comment: null)],
      );
      h.api.fake.hold = true;

      h.chat.setMessageFeedback('m1', _positive);

      // Optimistic: visible before the server answered.
      expect(h.feedbackOf('m1')?.rating, MessageRating.positive);
      expect(h.feedback.isPending('m1'), isTrue, reason: 'call in flight');
      await h.settle();
      expect(h.api.fake.calls, [
        (
          conversationId: 'conv-1',
          messageId: 'm1',
          rating: 'positive',
          comment: null,
        ),
      ]);
      expect(h.feedback.isPending('m1'), isTrue);

      h.api.fake.held.single.succeed();
      await h.settle();

      expect(h.feedback.isPending('m1'), isFalse, reason: 'settled');
      expect(h.feedbackOf('m1')?.rating, MessageRating.positive);
      expect(h.state.error, isNull);
    });

    test('clearing the vote sends DELETE', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );

      h.chat.setMessageFeedback('m1', null);
      expect(h.feedbackOf('m1'), isNull);
      await h.settle();

      expect(h.api.fake.calls.single.rating, isNull, reason: 'DELETE');
      expect(h.feedback.isPending('m1'), isFalse);
    });

    test('a re-tap of the same rating calls DELETE', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );

      h.chat.setMessageRating('m1', null);
      await h.settle();

      expect(h.feedbackOf('m1'), isNull);
      expect(h.api.fake.calls.single.rating, isNull);
    });

    test('an unknown message id is ignored', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );

      h.chat.setMessageFeedback('nope', _negative);
      await h.settle();

      expect(h.api.fake.calls, isEmpty);
      expect(h.feedback.isPending('nope'), isFalse);
    });
  });

  group('errors', () {
    test(
      'a failure restores rating and comment and reports an error',
      () async {
        final h = await _harness(
          history: [(id: 'm1', rating: 'negative', comment: 'wrong file')],
        );
        h.api.fake.hold = true;

        h.chat.setMessageFeedback('m1', _positive);
        await h.settle();
        expect(h.feedbackOf('m1')?.rating, MessageRating.positive);

        h.api.fake.held.single.fail();
        await h.settle();

        expect(h.feedbackOf('m1')?.rating, MessageRating.negative);
        expect(h.feedbackOf('m1')?.comment, 'wrong file');
        expect(h.state.error, isNotNull);
        expect(h.feedback.isPending('m1'), isFalse);
      },
    );

    test('rollback under the queue restores the confirmed value', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );
      h.api.fake.hold = true;

      h.chat.setMessageFeedback('m1', _negative);
      h.chat.setMessageFeedback('m1', null);
      await h.settle();
      expect(h.api.fake.calls, hasLength(1), reason: 'second call is queued');

      h.api.fake.held.single.fail();
      await h.settle();

      expect(
        h.api.fake.calls,
        hasLength(1),
        reason: 'the queued clear is never sent',
      );
      expect(h.feedbackOf('m1')?.rating, MessageRating.positive);
      expect(h.state.error, isNotNull);
    });
  });

  group('queue order', () {
    test('two quick toggles apply in order and end with no feedback', () async {
      final h = await _harness(history: _unrated);
      h.api.fake.hold = true;

      h.chat.setMessageRating('m1', MessageRating.positive);
      h.chat.setMessageRating('m1', null);
      await h.settle();

      expect(h.api.fake.calls.map((c) => c.rating), ['positive']);

      h.api.fake.held[0].succeed();
      await h.settle();
      expect(h.api.fake.calls.map((c) => c.rating), ['positive', null]);

      h.api.fake.held[1].succeed();
      await h.settle();

      expect(h.feedbackOf('m1'), isNull);
      expect(h.feedback.isPending('m1'), isFalse);
    });
  });

  group('history reconcile', () {
    test(
      'pending beats history: a reload during a PUT keeps the optimistic value',
      () async {
        final h = await _harness(
          history: [(id: 'm1', rating: 'positive', comment: null)],
        );
        h.api.fake.hold = true;

        h.chat.setMessageFeedback('m1', _negative);
        await h.settle();

        // The history still says `positive`.
        await h.chat.loadConversation('conv-1');

        expect(h.feedbackOf('m1')?.rating, MessageRating.negative);

        h.api.fake.held.single.succeed();
        await h.settle();
        expect(h.feedbackOf('m1')?.rating, MessageRating.negative);
      },
    );

    test('a settled message takes its feedback from history again', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );
      h.chat.setMessageFeedback('m1', _negative);
      await h.settle();

      h.api.fake.history['conv-1'] = [
        (id: 'm1', rating: 'positive', comment: 'from another device'),
      ];
      await h.chat.loadConversation('conv-1');

      expect(h.feedbackOf('m1')?.comment, 'from another device');
    });
  });

  group('stale callbacks', () {
    test('a failure after switching conversation changes nothing', () async {
      final h = await _harness(
        history: [(id: 'm1', rating: 'positive', comment: null)],
      );
      h.api.fake.history['conv-2'] = [
        (id: 'm2', rating: 'negative', comment: null),
      ];
      h.api.fake.hold = true;

      h.chat.setMessageFeedback('m1', _negative);
      await h.settle();

      await h.chat.loadConversation('conv-2');
      final before = h.state;

      h.api.fake.held.single.fail();
      await h.settle();

      expect(h.state.conversationId, 'conv-2');
      expect(h.state.error, isNull, reason: 'no error for another thread');
      expect(h.state.messages.single.feedback?.rating, MessageRating.negative);
      expect(identical(h.state, before), isTrue, reason: 'state untouched');
      expect(h.feedback.isPending('m1'), isFalse);
    });
  });

  group('comments', () {
    test('choosing either rating opens the comment field', () async {
      final h = await _harness(
        history: [
          (id: 'm1', rating: 'positive', comment: null),
          (id: 'm2', rating: 'negative', comment: null),
        ],
      );

      h.chat.setMessageRating('m1', MessageRating.positive);
      h.chat.setMessageRating('m2', MessageRating.negative);

      expect(h.feedback.commentFieldRating('m1'), MessageRating.positive);
      expect(h.feedback.commentFieldRating('m2'), MessageRating.negative);
    });

    test('submitting PUTs {rating, comment} for positive', () async {
      final h = await _harness(history: _unrated);
      h.chat.setMessageRating('m1', MessageRating.positive);
      await h.settle();

      h.chat.submitMessageFeedbackComment('m1', '  clear and short ');
      await h.settle();

      expect(h.api.fake.calls.last.rating, 'positive');
      expect(h.api.fake.calls.last.comment, 'clear and short');
      expect(h.feedbackOf('m1')?.comment, 'clear and short');
      expect(h.feedback.commentFieldRating('m1'), isNull, reason: 'closed');
    });

    test('submitting PUTs {rating, comment} for negative', () async {
      final h = await _harness(history: _unrated);
      h.chat.setMessageRating('m1', MessageRating.negative);
      await h.settle();

      h.chat.submitMessageFeedbackComment('m1', 'wrong file');
      await h.settle();

      expect(h.api.fake.calls.last.rating, 'negative');
      expect(h.api.fake.calls.last.comment, 'wrong file');
    });

    test(
      'switching rating drops the stored comment and resets the field',
      () async {
        final h = await _harness(
          history: [(id: 'm1', rating: 'negative', comment: 'wrong file')],
        );

        h.chat.setMessageRating('m1', MessageRating.positive);
        await h.settle();

        expect(h.api.fake.calls.single.rating, 'positive');
        expect(h.api.fake.calls.single.comment, isNull);
        expect(h.feedbackOf('m1')?.comment, isNull);
        expect(h.feedback.commentFieldRating('m1'), MessageRating.positive);
      },
    );

    test('dismissing the field sends nothing and keeps the rating', () async {
      final h = await _harness(history: _unrated);
      h.chat.setMessageRating('m1', MessageRating.negative);
      await h.settle();
      final callsBefore = h.api.fake.calls.length;

      h.chat.dismissMessageFeedbackComment('m1');
      await h.settle();

      expect(h.api.fake.calls, hasLength(callsBefore));
      expect(h.feedbackOf('m1')?.rating, MessageRating.negative);
      expect(h.feedback.commentFieldRating('m1'), isNull);
    });

    test('clearing the vote closes the field', () async {
      final h = await _harness(history: _unrated);
      h.chat.setMessageRating('m1', MessageRating.negative);
      h.chat.setMessageRating('m1', null);

      expect(h.feedback.commentFieldRating('m1'), isNull);
    });

    test('a comment submit is disabled while a call is in flight', () async {
      final h = await _harness(history: _unrated);
      h.api.fake.hold = true;
      h.chat.setMessageRating('m1', MessageRating.negative);
      await h.settle();

      // 7b disables Send from isPending; the notifier also refuses.
      h.chat.submitMessageFeedbackComment('m1', 'late');
      await h.settle();

      expect(h.api.fake.calls, hasLength(1));
      expect(h.feedback.commentFieldRating('m1'), MessageRating.negative);
    });
  });
}
