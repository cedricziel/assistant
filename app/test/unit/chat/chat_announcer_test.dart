// Policy tests for ChatAnnouncer (openspec chat-a11y-and-code-blocks, PR C2a).
//
// The announcer is a pure policy over lifecycle events and a clock, so every
// case runs on FakeAsync and asserts `(time, text, assertive)` triples.

import 'package:fake_async/fake_async.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_announcer.dart';

typedef Spoken = (Duration, String, bool);

final _epoch = DateTime.utc(2026, 1, 1);

Duration _s(num seconds) => Duration(milliseconds: (seconds * 1000).round());

/// Runs [body] on a fake clock with an announcer for the Persona "Ada".
///
/// [emit] feeds an event stamped with the current fake time; the returned
/// list holds what reached the sink and when.
List<Spoken> _run(
  void Function(
    FakeAsync async,
    ChatAnnouncer announcer,
    void Function(ChatLifecycleEvent Function(DateTime at)) emit,
  )
  body,
) {
  final spoken = <Spoken>[];
  FakeAsync().run((async) {
    DateTime now() => _epoch.add(async.elapsed);
    final announcer = ChatAnnouncer(
      personaName: () => 'Ada',
      sink: (a) => spoken.add((async.elapsed, a.text, a.assertive)),
      now: now,
    );
    body(async, announcer, (build) => announcer.handle(build(now())));
    announcer.dispose();
  });
  return spoken;
}

ChatLifecycleEvent _started(DateTime at) => TurnStarted(at: at);
ChatLifecycleEvent _stopped(DateTime at) => TurnStopped(at: at);
ChatLifecycleEvent _stalled(DateTime at) => TurnStalled(at: at);
ChatLifecycleEvent _resolved(DateTime at) => TurnInputResolved(at: at);

ChatLifecycleEvent Function(DateTime) _completed(
  String content, [
  TurnSource source = TurnSource.live,
]) =>
    (at) => TurnCompleted(content: content, source: source, at: at);

ChatLifecycleEvent Function(DateTime) _awaiting(String text) =>
    (at) => TurnAwaitingInput(announcement: text, at: at);

void main() {
  group('ChatAnnouncer', () {
    test('start then completion at 4 s announces both, throttled to 1.5 s', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        async.elapse(_s(4));
        emit(_completed('**Hello** there'));
        async.elapse(_s(2));
      });
      expect(spoken, [
        (_s(1.5), 'Ada is responding', false),
        (_s(5.5), 'Ada replied: Hello there', false),
      ]);
    });

    test(
      'rapid start and completion coalesce into the completion at 1.5 s',
      () {
        final spoken = _run((async, announcer, emit) {
          emit(_started);
          async.elapse(_s(0.8));
          emit(_completed('Done'));
          async.elapse(_s(3));
        });
        expect(spoken, [(_s(1.5), 'Ada replied: Done', false)]);
      },
    );

    test('failure is assertive at once and drops the held start', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        async.elapse(_s(0.5));
        emit((at) => TurnFailed(message: 'rate limited', at: at));
        async.elapse(_s(3));
      });
      expect(spoken, [(_s(0.5), 'Response failed: rate limited', true)]);
    });

    test('a 1,000-character reply is excerpted at a word boundary', () {
      final reply = List.filled(200, 'word').join(' ');
      expect(reply.length, greaterThan(900));
      final spoken = _run((async, announcer, emit) {
        emit(_completed(reply));
        async.elapse(_s(2));
      });
      const prefix = 'Ada replied: ';
      final excerpt = spoken.single.$2.substring(prefix.length);
      expect(excerpt, endsWith('…'));
      final body = excerpt.substring(0, excerpt.length - 1);
      expect(body.length, lessThanOrEqualTo(200));
      expect(
        body.split(' ').every((w) => w == 'word'),
        isTrue,
        reason: 'cut must fall between words, never inside one',
      );
    });

    test('a reply that fits in 200 characters has no ellipsis', () {
      final reply = 'a' * 200;
      final spoken = _run((async, announcer, emit) {
        emit(_completed(reply));
        async.elapse(_s(2));
      });
      expect(spoken.single.$2, 'Ada replied: $reply');
    });

    test('TurnStopped announces "Response stopped"', () {
      final spoken = _run((async, announcer, emit) {
        emit(_stopped);
        async.elapse(_s(2));
      });
      expect(spoken, [(_s(1.5), 'Response stopped', false)]);
    });

    test('TurnStalled twice in one turn announces "Still working" once', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        async.elapse(_s(2));
        emit(_stalled);
        async.elapse(_s(2));
        emit(_stalled);
        async.elapse(_s(2));
      });
      expect(spoken.map((s) => s.$2), ['Ada is responding', 'Still working']);
    });

    test('TurnStalled while awaiting input announces nothing extra', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        async.elapse(_s(2));
        emit(_awaiting('Approval needed'));
        async.elapse(_s(2));
        emit(_stalled);
        async.elapse(_s(2));
      });
      expect(spoken.map((s) => s.$2), ['Ada is responding', 'Approval needed']);
    });

    test('TurnStalled after TurnInputResolved is announced', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        emit(_awaiting('Approval needed'));
        emit(_resolved);
        emit(_stalled);
        async.elapse(_s(2));
      });
      expect(spoken.map((s) => s.$2), ['Still working']);
    });

    test('TurnCompleted from a replay is announced like a live one', () {
      final spoken = _run((async, announcer, emit) {
        emit(_completed('Welcome back', TurnSource.replay));
        async.elapse(_s(2));
      });
      expect(spoken, [(_s(1.5), 'Ada replied: Welcome back', false)]);
    });

    test('an abandoned approval does not suppress the next turn stall', () {
      final spoken = _run((async, announcer, emit) {
        emit(_started);
        emit(_awaiting('Approval needed'));
        async.elapse(_s(2));
        // The user edited the message; the server cancelled the approval and
        // no TurnInputResolved arrives. The next turn starts fresh.
        emit(_started);
        async.elapse(_s(2));
        emit(_stalled);
        async.elapse(_s(2));
      });
      expect(spoken.map((s) => s.$2), [
        'Approval needed',
        'Ada is responding',
        'Still working',
      ]);
    });

    test('the persona name is read when the event is handled', () {
      var name = 'Ada';
      final spoken = <String>[];
      FakeAsync().run((async) {
        final announcer = ChatAnnouncer(
          personaName: () => name,
          sink: (a) => spoken.add(a.text),
          now: () => _epoch.add(async.elapsed),
        );
        name = 'Grace';
        announcer.handle(TurnStarted(at: _epoch));
        async.elapse(_s(2));
        announcer.dispose();
      });
      expect(spoken, ['Grace is responding']);
    });

    test(
      'say() is throttled; an assertive say() is immediate and drops it',
      () {
        final spoken = _run((async, announcer, emit) {
          announcer.say('Code copied');
          async.elapse(_s(0.2));
          announcer.say('Approval needed', assertive: true);
          async.elapse(_s(3));
        });
        expect(spoken, [(_s(0.2), 'Approval needed', true)]);
      },
    );

    test(
      'a newer polite announcement replaces the held one, keeping the deadline',
      () {
        final spoken = _run((async, announcer, emit) {
          announcer.say('first');
          async.elapse(_s(1));
          announcer.say('second');
          async.elapse(_s(3));
        });
        expect(spoken, [(_s(1.5), 'second', false)]);
      },
    );

    test('time passing without lifecycle events produces nothing', () {
      // Token frames are not lifecycle events; the policy has no entry point
      // for them, so a long stream is silent by construction.
      final spoken = _run((async, announcer, emit) {
        for (var i = 0; i < 200; i++) {
          async.elapse(const Duration(milliseconds: 10));
        }
        async.elapse(_s(2));
      });
      expect(spoken, isEmpty);
    });

    test('dispose cancels a held announcement', () {
      final spoken = <String>[];
      FakeAsync().run((async) {
        final announcer = ChatAnnouncer(
          personaName: () => 'Ada',
          sink: (a) => spoken.add(a.text),
          now: () => _epoch.add(async.elapsed),
        );
        announcer.handle(TurnStarted(at: _epoch));
        announcer.dispose();
        async.elapse(_s(2));
      });
      expect(spoken, isEmpty);
    });
  });

  group('markdownToPlainText', () {
    test('strips emphasis, keeps link text, replaces fences', () {
      expect(
        markdownToPlainText('**Bold** and _it_ and `code`'),
        'Bold and it and code',
      );
      expect(
        markdownToPlainText('See [the docs](https://x.y/z) now'),
        'See the docs now',
      );
      expect(
        markdownToPlainText('Before\n```dart\nvar a = 1;\n```\nAfter'),
        'Before code block After',
      );
      expect(
        markdownToPlainText('# Title\n\n- one\n- two\n1. three'),
        'Title one two three',
      );
      expect(
        markdownToPlainText('> quoted\n\n![alt text](http://i/x.png)'),
        'quoted alt text',
      );
    });

    test('leaves snake_case identifiers and bare asterisks alone', () {
      expect(
        markdownToPlainText('use web_search_tool for 2 * 3'),
        'use web_search_tool for 2 * 3',
      );
    });

    test('collapses whitespace', () {
      expect(markdownToPlainText('a\n\n\n  b\t c'), 'a b c');
    });
  });

  group('ChatAnnouncementSink', () {
    TestWidgetsFlutterBinding.ensureInitialized();

    test('sends an announce message on the accessibility channel', () async {
      final sent = <Map<Object?, Object?>>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockDecodedMessageHandler<Object?>(SystemChannels.accessibility, (
            message,
          ) async {
            sent.add(message! as Map<Object?, Object?>);
            return null;
          });
      addTearDown(
        () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockDecodedMessageHandler<Object?>(
              SystemChannels.accessibility,
              null,
            ),
      );

      const ChatAnnouncementSink()(const Announcement('Hello'));
      await Future<void>.delayed(Duration.zero);

      expect(sent.single['type'], 'announce');
      expect((sent.single['data']! as Map)['message'], 'Hello');
    });

    test('chatAnnouncerProvider routes through the overridable sink', () {
      final received = <Announcement>[];
      final container = ProviderContainer(
        overrides: [
          chatAnnouncementSinkProvider.overrideWithValue(received.add),
        ],
      );
      addTearDown(container.dispose);

      container.read(chatAnnouncerProvider).say('Code copied', assertive: true);

      expect(received.single.text, 'Code copied');
      expect(received.single.assertive, isTrue);
    });
  });
}
