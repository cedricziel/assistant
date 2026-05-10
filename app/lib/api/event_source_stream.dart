// Public surface for the conversation-list SSE stream.
//
// On web, the implementation uses the browser's native `EventSource` API
// (see `event_source_stream_web.dart`). On native platforms the adapter is
// a no-op stub that throws if invoked — `streamConversations()` only
// delegates to this module on `kIsWeb`.
//
// Exposed publicly so tests can inject a fake [EventSourceAdapter] without
// touching the browser API.

import 'dart:async';
import 'dart:convert';

import 'models/stream_event.dart';

/// One SSE message: an event name + its data payload.
class EventSourceMessage {
  const EventSourceMessage({required this.event, required this.data});
  final String event;
  final String data;
}

/// Minimal interface for the underlying transport. The web implementation
/// wraps `package:web`'s `EventSource`. Tests inject a fake.
abstract class EventSourceAdapter {
  Stream<EventSourceMessage> get messages;
  void close();
}

/// Convert a [Stream<EventSourceMessage>] into a [Stream<ConversationListEvent>].
///
/// Closes the underlying adapter when the returned subscription is cancelled.
Stream<ConversationListEvent> openConversationListStream({
  required EventSourceAdapter adapter,
}) {
  late StreamController<ConversationListEvent> controller;
  StreamSubscription<EventSourceMessage>? sub;

  controller = StreamController<ConversationListEvent>(
    onListen: () {
      sub = adapter.messages.listen(
        (msg) {
          final event = _decode(msg);
          if (event != null) controller.add(event);
        },
        onError: controller.addError,
        onDone: controller.close,
      );
    },
    onCancel: () async {
      adapter.close();
      await sub?.cancel();
    },
  );

  return controller.stream;
}

ConversationListEvent? _decode(EventSourceMessage msg) {
  try {
    switch (msg.event) {
      case 'snapshot':
        final json = jsonDecode(msg.data) as List<dynamic>;
        return ConversationSnapshotEvent.fromJson(json);
      case 'upserted':
        final json = jsonDecode(msg.data) as Map<String, dynamic>;
        return ConversationUpsertedEvent.fromJson(json);
      case 'deleted':
        final json = jsonDecode(msg.data) as Map<String, dynamic>;
        return ConversationDeletedEvent.fromJson(json);
      default:
        return null; // unknown event types — ignore (forward-compatible)
    }
  } catch (_) {
    return null; // malformed payload — drop, don't break the stream
  }
}
