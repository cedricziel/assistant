import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/api/models/surface_action.dart';
import 'package:dio/dio.dart';

/// One call to [FakeFollowUpApi.streamEventsFrom], with the controller a test
/// pushes replay frames into.
class FakeReplay {
  FakeReplay({
    required this.conversationId,
    required this.runId,
    required this.since,
    required this.cancelToken,
  });

  final String conversationId;
  final String runId;
  final int since;
  final CancelToken? cancelToken;
  final controller = StreamController<StreamEvent>();

  /// Whether the consumer cancelled its subscription.
  bool cancelled = false;
}

/// [ApiClient] whose POST and replay streams a test drives by hand.
class FakeFollowUpApi extends ApiClient {
  FakeFollowUpApi() : super(baseUrl: 'http://fake', token: 'fake');

  final _posts = <StreamController<StreamEvent>>[];

  /// Every replay subscription opened, in call order.
  final replays = <FakeReplay>[];

  /// Queue the stream the next `streamMessages` call returns.
  StreamController<StreamEvent> enqueuePost() {
    final ctrl = StreamController<StreamEvent>();
    _posts.add(ctrl);
    return ctrl;
  }

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
    SurfaceActionSubmission? surfaceAction,
  }) => _posts.isEmpty ? const Stream.empty() : _posts.removeAt(0).stream;

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) {
    final replay = FakeReplay(
      conversationId: conversationId,
      runId: runId,
      since: since,
      cancelToken: cancelToken,
    );
    replay.controller.onCancel = () => replay.cancelled = true;
    replays.add(replay);
    return replay.controller.stream;
  }

  @override
  ConversationsApi get conversations => FakeConversationsApi();

  @override
  Future<List<InteractionDetail>> listConversationInteractions(
    String conversationId, {
    InteractionStatusView? status,
  }) async => const [];
}

/// `getConversation` answers with one finished assistant message.
class FakeConversationsApi extends ConversationsApi {
  FakeConversationsApi() : super(Dio(), standardSerializers);

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
  }) async => Response(
    requestOptions: RequestOptions(path: '/api/conversations/$id'),
    statusCode: 200,
    data: ConversationDetail(
      (b) => b
        ..id = id
        ..title = 'T'
        ..createdAt = DateTime(2024, 1, 1)
        ..updatedAt = DateTime(2024, 1, 1)
        ..messages.replace([
          MessageSummary(
            (m) => m
              ..id = 'hist-1'
              ..role = 'assistant'
              ..content = 'old answer'
              ..createdAt = DateTime(2024, 1, 1)
              ..turn = 0
              ..ttsAvailable = false,
          ),
        ]),
    ),
  );
}
