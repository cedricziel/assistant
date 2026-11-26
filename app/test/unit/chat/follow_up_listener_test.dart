import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/follow_up_listener.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';

import 'follow_up_test_support.dart';

FollowUpListener _listener(
  FakeFollowUpApi api,
  List<List<String>> received, {
  String messageId = 'm1',
}) => FollowUpListener(
  api: api,
  conversationId: 'conv-1',
  runId: 'run-1',
  since: 7,
  messageId: messageId,
  onSuggestions: received.add,
)..start();

void main() {
  test('opens the replay with the captured ids and its own cancel token', () {
    final api = FakeFollowUpApi();
    _listener(api, []);

    final replay = api.replays.single;
    expect(replay.conversationId, 'conv-1');
    expect(replay.runId, 'run-1');
    expect(replay.since, 7);
    expect(replay.cancelToken, isNotNull);
  });

  test(
    'delivers suggestions for its message, then closes the stream',
    () async {
      final api = FakeFollowUpApi();
      final received = <List<String>>[];
      _listener(api, received);

      api.replays.single.controller.add(
        const SuggestionsEvent(messageId: 'm1', prompts: ['a', 'b']),
      );
      await Future<void>.delayed(Duration.zero);

      expect(received, [
        ['a', 'b'],
      ]);
      expect(api.replays.single.cancelled, isTrue);
    },
  );

  test('ignores suggestions for another message', () async {
    final api = FakeFollowUpApi();
    final received = <List<String>>[];
    _listener(api, received);

    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'other', prompts: ['a']),
    );
    await Future<void>.delayed(Duration.zero);

    expect(received, isEmpty);
  });

  test('ignores an empty suggestion list', () async {
    final api = FakeFollowUpApi();
    final received = <List<String>>[];
    _listener(api, received);

    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: []),
    );
    await Future<void>.delayed(Duration.zero);

    expect(received, isEmpty);
  });

  test('cancel closes the subscription and the cancel token', () async {
    final api = FakeFollowUpApi();
    final received = <List<String>>[];
    final listener = _listener(api, received);

    listener.cancel();
    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['late']),
    );
    await Future<void>.delayed(Duration.zero);

    expect(api.replays.single.cancelled, isTrue);
    expect(api.replays.single.cancelToken!.isCancelled, isTrue);
    expect(received, isEmpty);
  });

  test(
    'a replay that never sends suggestions is cancelled after the window',
    () {
      fakeAsync((async) {
        final api = FakeFollowUpApi();
        _listener(api, []);

        async.elapse(kSuggestionsListenWindow - const Duration(seconds: 1));
        expect(api.replays.single.cancelled, isFalse);

        async.elapse(const Duration(seconds: 1));
        expect(api.replays.single.cancelled, isTrue);
      });
    },
  );

  test('the listen window is the server maximum plus network margin', () {
    expect(kSuggestionsListenWindow, const Duration(seconds: 20));
  });

  test('a stream error ends the listener quietly', () async {
    final api = FakeFollowUpApi();
    final received = <List<String>>[];
    _listener(api, received);

    api.replays.single.controller.addError(StateError('boom'));
    await Future<void>.delayed(Duration.zero);

    expect(received, isEmpty);
    expect(api.replays.single.cancelled, isTrue);
  });
}
