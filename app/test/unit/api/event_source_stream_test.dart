// Asserts the EventSource adapter shape: given a fake EventSource that
// dispatches snapshot/upserted/deleted events, the adapter yields the
// matching ConversationListEvent types.
//
// Spec: openspec/changes/web-sse-eventsource/specs/web-sse-eventsource/spec.md

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/event_source_stream.dart';
import 'package:assistant_app/api/models/stream_event.dart';

class _FakeEventSourceAdapter implements EventSourceAdapter {
  final List<EventSourceMessage Function()> _scripted = [];
  bool closed = false;

  void scheduleSnapshot(List<Map<String, dynamic>> conversations) {
    _scripted.add(
      () => EventSourceMessage(
        event: 'snapshot',
        data: '[${conversations.map(_jsonOfMap).join(',')}]',
      ),
    );
  }

  void scheduleUpserted(Map<String, dynamic> conversation) {
    _scripted.add(
      () =>
          EventSourceMessage(event: 'upserted', data: _jsonOfMap(conversation)),
    );
  }

  void scheduleDeleted(String conversationId) {
    _scripted.add(
      () => EventSourceMessage(
        event: 'deleted',
        data: '{"conversation_id":"$conversationId"}',
      ),
    );
  }

  String _jsonOfMap(Map<String, dynamic> m) {
    final entries = m.entries
        .map((e) {
          final v = e.value;
          if (v is String) return '"${e.key}":"$v"';
          return '"${e.key}":$v';
        })
        .join(',');
    return '{$entries}';
  }

  @override
  Stream<EventSourceMessage> get messages async* {
    for (final make in _scripted) {
      yield make();
    }
  }

  @override
  void close() {
    closed = true;
  }
}

Map<String, dynamic> _convo(String id, String title) => {
  'id': id,
  'title': title,
  'agent_id': 'default',
  'user_id': 'usr_x',
  'created_at': '2026-05-10T00:00:00Z',
  'updated_at': '2026-05-10T00:00:00Z',
};

void main() {
  test('snapshot event yields ConversationSnapshotEvent', () async {
    final fake = _FakeEventSourceAdapter()
      ..scheduleSnapshot([_convo('a', 'Alpha'), _convo('b', 'Beta')]);

    final events = await openConversationListStream(adapter: fake).toList();

    expect(events.length, 1);
    expect(events.first, isA<ConversationSnapshotEvent>());
    final snap = events.first as ConversationSnapshotEvent;
    expect(snap.conversations.length, 2);
    expect(snap.conversations[0].id, 'a');
    expect(snap.conversations[1].id, 'b');
  });

  test('upserted and deleted events yield matching types', () async {
    final fake = _FakeEventSourceAdapter()
      ..scheduleSnapshot([_convo('a', 'Alpha')])
      ..scheduleUpserted(_convo('a', 'Alpha v2'))
      ..scheduleDeleted('a');

    final events = await openConversationListStream(adapter: fake).toList();

    expect(events[0], isA<ConversationSnapshotEvent>());
    expect(events[1], isA<ConversationUpsertedEvent>());
    expect(
      (events[1] as ConversationUpsertedEvent).conversation.title,
      'Alpha v2',
    );
    expect(events[2], isA<ConversationDeletedEvent>());
    expect((events[2] as ConversationDeletedEvent).conversationId, 'a');
  });

  test('subscription cancel closes the underlying adapter', () async {
    final fake = _FakeEventSourceAdapter()
      ..scheduleSnapshot([_convo('a', 'Alpha')]);

    final sub = openConversationListStream(adapter: fake).listen((_) {});
    await sub.cancel();

    expect(
      fake.closed,
      isTrue,
      reason: 'cancel must close the underlying EventSource',
    );
  });
}
