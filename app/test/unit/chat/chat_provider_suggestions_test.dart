import 'dart:async';

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_stream_session.dart';
import 'package:assistant_app/features/chat/follow_up_listener.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'follow_up_test_support.dart';

class _Harness {
  _Harness(this.api, this.async) {
    container = ProviderContainer(
      overrides: [apiClientProvider.overrideWithValue(api)],
    );
    // chatProvider is autoDispose: a listener keeps it alive across timers.
    container.listen(chatProvider, (_, _) {});
    notifier = container.read(chatProvider.notifier);
    async.flushMicrotasks();
    notifier.setConversationId('conv-1');
  }

  final FakeFollowUpApi api;
  final FakeAsync async;
  late final ProviderContainer container;
  late final ChatNotifier notifier;

  ChatState get state => container.read(chatProvider).value!;

  void flush() => async.flushMicrotasks();

  /// Send [text] and finish its POST stream with run [runId] and a `done`.
  void completeTurn(
    String text,
    StreamController<StreamEvent> post, {
    String runId = 'run-1',
    String? messageId = 'm1',
    int seq = 5,
  }) {
    unawaited(notifier.sendMessage(text));
    flush();
    post.add(RunStartedEvent(runId));
    post.add(
      DoneEvent(
        role: 'assistant',
        content: 'ok',
        messageId: messageId,
        sequenceId: seq,
      ),
    );
    flush();
  }
}

void _scenario(
  String name,
  void Function(_Harness h, FakeFollowUpApi api) body,
) {
  test(name, () {
    fakeAsync((async) {
      final api = FakeFollowUpApi();
      final h = _Harness(api, async);
      body(h, api);
      h.container.dispose();
    });
  });
}

void main() {
  _scenario('done unlocks the composer and chips follow on the replay', (
    h,
    api,
  ) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);

    expect(h.state.isSending, isFalse);
    expect(api.replays, hasLength(1));
    expect(api.replays.single.runId, 'run-1');
    expect(api.replays.single.since, 6, reason: 'done.sequenceId + 1');
    expect(h.state.suggestions, isEmpty);

    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['one', 'two']),
    );
    h.flush();

    expect(h.state.suggestions, ['one', 'two']);
  });

  _scenario('a queued send drops late suggestions and cancels the listener', (
    h,
    api,
  ) {
    final post1 = api.enqueuePost();
    final post2 = api.enqueuePost();
    unawaited(h.notifier.sendMessage('first'));
    h.flush();
    unawaited(h.notifier.sendMessage('second'));
    h.flush();

    post1
      ..add(const RunStartedEvent('run-1'))
      ..add(
        const DoneEvent(
          role: 'assistant',
          content: 'r1',
          messageId: 'm1',
          sequenceId: 5,
        ),
      );
    h.flush();

    final listener = api.replays.single;
    expect(listener.cancelled, isTrue, reason: 'second send started');

    listener.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['late']),
    );
    h.flush();
    expect(h.state.suggestions, isEmpty);

    // Run 2's own cursor is untouched by run 1's late frame.
    post2.add(const RunStartedEvent('run-2'));
    h.flush();
    expect(api.replays, hasLength(1));
    post2.add(
      const DoneEvent(
        role: 'assistant',
        content: 'r2',
        messageId: 'm2',
        sequenceId: 3,
      ),
    );
    h.flush();
    expect(api.replays, hasLength(2));
    expect(api.replays.last.runId, 'run-2');
    expect(api.replays.last.since, 4);
  });

  _scenario('sendMessage clears chips and cancels the listener', (h, api) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);
    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['one']),
    );
    h.flush();
    expect(h.state.suggestions, ['one']);

    api.enqueuePost();
    unawaited(h.notifier.sendMessage('next'));
    h.flush();

    expect(h.state.suggestions, isEmpty);
    expect(api.replays.single.cancelled, isTrue);
  });

  _scenario('loadConversation clears chips and cancels the listener', (h, api) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);
    api.replays.single.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['one']),
    );
    h.flush();

    unawaited(h.notifier.loadConversation('conv-2'));
    h.flush();

    expect(h.state.suggestions, isEmpty);
    expect(api.replays.single.cancelled, isTrue);
  });

  _scenario('clearConversation clears chips and cancels the listener', (
    h,
    api,
  ) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);
    expect(api.replays.single.cancelled, isFalse);

    h.notifier.clearConversation();

    expect(h.state.suggestions, isEmpty);
    expect(api.replays.single.cancelled, isTrue);
  });

  _scenario('disposing the notifier cancels the listener', (h, api) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);

    h.container.dispose();

    expect(api.replays.single.cancelled, isTrue);
  });

  _scenario('a replay without suggestions is cancelled after the window', (
    h,
    api,
  ) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post);
    expect(api.replays.single.cancelled, isFalse);

    h.async.elapse(kSuggestionsListenWindow);

    expect(api.replays.single.cancelled, isTrue);
    expect(h.state.suggestions, isEmpty);
  });

  _scenario('a done without message id starts no listener', (h, api) {
    final post = api.enqueuePost();
    h.completeTurn('hi', post, messageId: null);

    expect(h.state.isSending, isFalse);
    expect(api.replays, isEmpty);
  });

  _scenario('reopening a conversation shows no chips and starts no listener', (
    h,
    api,
  ) {
    unawaited(h.notifier.loadConversation('conv-2'));
    h.flush();

    expect(h.state.messages.single.id, 'hist-1');
    expect(h.state.suggestions, isEmpty);
    expect(api.replays, isEmpty);
  });

  _scenario('the replay path also starts the listener at done', (h, api) {
    final post = api.enqueuePost();
    unawaited(h.notifier.sendMessage('hi'));
    h.flush();
    post.add(const RunStartedEvent('run-1'));
    h.flush();
    post.addError(TimeoutException('dropped'));
    h.flush();
    h.async.elapse(ChatStreamSession.retryDelay(0));
    h.flush();

    // The POST stream dropped: the notifier replays run-1 from the log.
    expect(api.replays, hasLength(1));
    api.replays.first.controller.add(
      const DoneEvent(
        role: 'assistant',
        content: 'ok',
        messageId: 'm1',
        sequenceId: 9,
      ),
    );
    h.flush();

    expect(h.state.isSending, isFalse);
    expect(api.replays, hasLength(2));
    expect(api.replays.last.since, 10);

    api.replays.last.controller.add(
      const SuggestionsEvent(messageId: 'm1', prompts: ['again']),
    );
    h.flush();
    expect(h.state.suggestions, ['again']);
  });
}
