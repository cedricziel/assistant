import 'dart:async';

import 'package:dio/dio.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';

/// How long [FollowUpListener] waits for the `suggestions` frame.
///
/// The server's suggestion phase is bounded by its `SUGGESTIONS_MAX_TIMEOUT_SECS`
/// (15 s); the extra 5 s absorbs network latency.
const kSuggestionsListenWindow = Duration(seconds: 15 + 5);

/// Waits on a finished run's replay stream for the server's follow-up
/// `suggestions` frame.
///
/// The POST stream closes at `done`, so suggestions arrive on a subscription
/// of their own. The listener works only from the values captured at `done`
/// and never touches the chat stream session: a late frame from an older run
/// cannot move a newer run's replay cursor.
class FollowUpListener {
  FollowUpListener({
    required this.api,
    required this.conversationId,
    required this.runId,
    required this.since,
    required this.messageId,
    required this.onSuggestions,
    this.wrapSource = _identity,
  });

  final ApiClient api;
  final String conversationId;
  final String runId;

  /// The replay cursor: one past the `done` frame.
  final int since;

  /// The assistant message the suggestions must follow up on.
  final String messageId;

  /// Called at most once, with a non-empty list of prompts.
  final void Function(List<String> prompts) onSuggestions;

  /// Applied to the replay stream, so the owner can end it early (the chat
  /// notifier passes its detach wrapper).
  final Stream<StreamEvent> Function(Stream<StreamEvent>) wrapSource;

  final _cancelToken = CancelToken();
  StreamSubscription<StreamEvent>? _subscription;
  Timer? _window;
  bool _closed = false;

  static Stream<StreamEvent> _identity(Stream<StreamEvent> source) => source;

  /// Open the replay stream and listen until suggestions, the stream's end,
  /// an error, [kSuggestionsListenWindow] or [cancel].
  void start() {
    _window = Timer(kSuggestionsListenWindow, cancel);
    _subscription = wrapSource(
      api.streamEventsFrom(
        conversationId,
        runId,
        since: since,
        cancelToken: _cancelToken,
      ),
    ).listen(_onEvent, onError: (Object _) => cancel(), onDone: cancel);
  }

  void _onEvent(StreamEvent event) {
    if (event is! SuggestionsEvent || event.messageId != messageId) return;
    final prompts = event.prompts;
    cancel();
    if (prompts.isNotEmpty) onSuggestions(prompts);
  }

  /// Stop listening and close the HTTP connection. Safe to call repeatedly.
  void cancel() {
    if (_closed) return;
    _closed = true;
    _window?.cancel();
    unawaited(_subscription?.cancel());
    _cancelToken.cancel();
  }
}
