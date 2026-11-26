// Tests for the SSE byte-stream parser.
//
// Key regression: the parser must accept Stream<Uint8List> (what Dio emits on
// native platforms) without throwing a runtime type error.  Previously the code
// used `.transform(utf8.decoder)` which fails on iOS with:
//   "type 'Utf8Decoder' is not a subtype of type
//    'StreamTransformer<Uint8List, String>'"

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encodes SSE text as a single [Uint8List] chunk (simulates what Dio emits on
/// native/iOS).
Stream<Uint8List> _uint8Stream(String sseText) async* {
  yield Uint8List.fromList(utf8.encode(sseText));
}

/// Encodes SSE text as a plain [List<int>] chunk (simulates web / older path).
Stream<List<int>> _listIntStream(String sseText) async* {
  yield utf8.encode(sseText);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

void main() {
  group('parseSseByteStream', () {
    const singleToken = 'event: token\ndata: hello\n\n';
    const doneEvent =
        'event: done\ndata: {"role":"assistant","content":"hi"}\n\n';
    const multiChunk =
        'event: token\ndata: foo\n\n'
        'event: token\ndata: bar\n\n'
        'event: done\ndata: {"role":"assistant","content":"foobar"}\n\n';

    // -----------------------------------------------------------------------
    // Regression: Uint8List stream must not throw on native platforms
    // -----------------------------------------------------------------------

    test('accepts Stream<Uint8List> without type error', () async {
      final events = await parseSseByteStream(_uint8Stream(singleToken))
          .toList();
      expect(events, hasLength(1));
      expect(events.first, isA<TokenEvent>());
    });

    test('accepts Stream<List<int>>', () async {
      final events = await parseSseByteStream(_listIntStream(singleToken))
          .toList();
      expect(events, hasLength(1));
      expect(events.first, isA<TokenEvent>());
    });

    // -----------------------------------------------------------------------
    // Token event parsing
    // -----------------------------------------------------------------------

    test('emits TokenEvent with correct token', () async {
      final events = await parseSseByteStream(_uint8Stream(singleToken))
          .toList();
      expect((events.first as TokenEvent).token, equals('hello'));
    });

    // -----------------------------------------------------------------------
    // Done event parsing
    // -----------------------------------------------------------------------

    test('emits DoneEvent with role and content from JSON', () async {
      final events = await parseSseByteStream(_uint8Stream(doneEvent)).toList();
      expect(events, hasLength(1));
      final done = events.first as DoneEvent;
      expect(done.role, equals('assistant'));
      expect(done.content, equals('hi'));
    });

    test('DoneEvent preserves messageId when present in JSON', () async {
      const raw =
          'event: done\ndata: {"role":"assistant","content":"ok","message_id":"abc-123"}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final done = events.first as DoneEvent;
      expect(
        done.messageId,
        equals('abc-123'),
        reason: 'messageId must be parsed from the done event JSON',
      );
    });

    test('DoneEvent falls back when data is not valid JSON', () async {
      const raw = 'event: done\ndata: not-json\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final done = events.first as DoneEvent;
      expect(done.role, equals('assistant'));
      expect(done.content, equals('not-json'));
    });

    // -----------------------------------------------------------------------
    // Multi-event stream
    // -----------------------------------------------------------------------

    test('emits multiple events in order', () async {
      final events = await parseSseByteStream(_uint8Stream(multiChunk))
          .toList();
      expect(events, hasLength(3));
      expect(events[0], isA<TokenEvent>());
      expect((events[0] as TokenEvent).token, equals('foo'));
      expect(events[1], isA<TokenEvent>());
      expect((events[1] as TokenEvent).token, equals('bar'));
      expect(events[2], isA<DoneEvent>());
    });

    // -----------------------------------------------------------------------
    // Multi-chunk stream (data arrives in separate byte chunks)
    // -----------------------------------------------------------------------

    test('handles event split across multiple chunks', () async {
      // Split the SSE message across two chunks, as a network might deliver.
      Stream<Uint8List> splitStream() async* {
        yield Uint8List.fromList(utf8.encode('event: token\n'));
        yield Uint8List.fromList(utf8.encode('data: world\n\n'));
      }

      final events = await parseSseByteStream(splitStream()).toList();
      expect(events, hasLength(1));
      expect((events.first as TokenEvent).token, equals('world'));
    });

    // -----------------------------------------------------------------------
    // Unknown / ignored event types
    // -----------------------------------------------------------------------

    test('ignores unknown event types', () async {
      const raw = 'event: ping\ndata: ignored\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, isEmpty);
    });

    // -----------------------------------------------------------------------
    // Follow-up suggestions
    // -----------------------------------------------------------------------

    test('parses a suggestions frame with its sequence id', () async {
      const raw =
          'id: 41\n'
          'event: suggestions\n'
          'data: {"message_id":"m-1","suggestions":'
          '[{"prompt":"Show me an example"},{"prompt":"Why?"}]}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final event = events.single as SuggestionsEvent;
      expect(event.messageId, equals('m-1'));
      expect(event.prompts, equals(['Show me an example', 'Why?']));
      expect(event.sequenceId, equals(41));
    });

    test('skips suggestion items without a prompt string', () async {
      const raw =
          'event: suggestions\n'
          'data: {"message_id":"m-1","suggestions":'
          '[{"prompt":"Keep"},{"title":"no prompt"},{"prompt":7},"bare"]}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect((events.single as SuggestionsEvent).prompts, equals(['Keep']));
    });

    test('drops a suggestions frame with malformed JSON', () async {
      const raw = 'event: suggestions\ndata: {not json\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, isEmpty);
    });

    test('drops a suggestions frame without a message_id', () async {
      const raw =
          'event: suggestions\n'
          'data: {"suggestions":[{"prompt":"Hi"}]}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, isEmpty);
    });

    test('ignores an unknown event and still parses later frames', () async {
      const raw =
          'event: future_thing\ndata: {"a":1}\n\n'
          'event: suggestions\n'
          'data: {"message_id":"m-1","suggestions":[{"prompt":"Hi"}]}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      expect(events.single, isA<SuggestionsEvent>());
    });

    // -----------------------------------------------------------------------
    // Status event with tool_call_id
    // -----------------------------------------------------------------------

    test('StatusEvent parses toolCallId from JSON data', () async {
      const raw =
          'event: status\ndata: {"message":"Calling tool: web-search","tool_call_id":"call_abc123"}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final status = events.first as StatusEvent;
      expect(status.message, equals('Calling tool: web-search'));
      expect(status.toolCallId, equals('call_abc123'));
    });

    test('StatusEvent handles plain-text data for backwards compat', () async {
      const raw = 'event: status\ndata: Calling tool: file-read\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final status = events.first as StatusEvent;
      expect(status.message, equals('Calling tool: file-read'));
      expect(status.toolCallId, isNull);
    });

    test('ToolResultEvent parses toolCallId from JSON data', () async {
      const raw =
          'event: tool_result\ndata: {"tool_name":"web-search","status":"ok","tool_call_id":"call_xyz789"}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final result = events.first as ToolResultEvent;
      expect(result.toolName, equals('web-search'));
      expect(result.toolCallId, equals('call_xyz789'));
    });

    test('ToolResultEvent has null toolCallId when not in JSON', () async {
      const raw =
          'event: tool_result\ndata: {"tool_name":"file-read","status":"ok"}\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final result = events.first as ToolResultEvent;
      expect(result.toolCallId, isNull);
    });

    // -----------------------------------------------------------------------
    // Turn interactions
    // -----------------------------------------------------------------------

    group('interaction events', () {
      Future<List<StreamEvent>> parse(String raw) =>
          parseSseByteStream(_uint8Stream(raw)).toList();

      const approval =
          'id: 7\n'
          'event: interaction_requested\n'
          'data: {"interaction_id":"i-1","kind":"tool_approval",'
          '"created_at":"2026-01-02T03:04:05Z",'
          '"expires_at":"2026-01-02T03:09:05Z","renders_card":true,'
          '"tool_call_id":"toolu_1","tool_name":"bash",'
          '"arguments":{"command":"git status"},"arguments_truncated":false,'
          '"reason":"no rule allows bash",'
          '"options":["allow_once","reject_once"]}\n\n';

      test('parses an approval request with its sequence id', () async {
        final event =
            (await parse(approval)).single as InteractionRequestedEvent;
        expect(event.interactionId, 'i-1');
        expect(event.kind, 'tool_approval');
        expect(event.createdAt, DateTime.utc(2026, 1, 2, 3, 4, 5));
        expect(event.expiresAt, DateTime.utc(2026, 1, 2, 3, 9, 5));
        expect(event.toolCallId, 'toolu_1');
        expect(event.rendersCard, isTrue);
        expect(event.request['tool_name'], 'bash');
        expect(event.request['arguments'], {'command': 'git status'});
        expect(event.request['options'], ['allow_once', 'reject_once']);
        expect(event.sequenceId, 7);
      });

      test('keeps the question list in the request fields', () async {
        const raw =
            'event: interaction_requested\n'
            'data: {"interaction_id":"i-2","kind":"question",'
            '"created_at":"2026-01-02T03:04:05Z",'
            '"expires_at":"2026-01-02T03:09:05Z",'
            '"questions":[{"id":"q1","prompt":"Which branch?"}]}\n\n';
        final event = (await parse(raw)).single as InteractionRequestedEvent;
        expect(event.kind, 'question');
        expect(event.toolCallId, isNull);
        expect((event.request['questions']! as List).single, {
          'id': 'q1',
          'prompt': 'Which branch?',
        });
      });

      test('keeps an unknown kind with its fields', () async {
        const raw =
            'event: interaction_requested\n'
            'data: {"interaction_id":"i-3","kind":"future_thing",'
            '"created_at":"2026-01-02T03:04:05Z",'
            '"expires_at":"2026-01-02T03:09:05Z","surface":"s1"}\n\n';
        final event = (await parse(raw)).single as InteractionRequestedEvent;
        expect(event.kind, 'future_thing');
        expect(event.request['surface'], 's1');
      });

      test('reads renders_card: false and defaults to true', () async {
        const hidden =
            'event: interaction_requested\n'
            'data: {"interaction_id":"i-4","kind":"surface_action",'
            '"created_at":"2026-01-02T03:04:05Z",'
            '"expires_at":"2026-01-02T03:09:05Z","renders_card":false}\n\n';
        const bare =
            'event: interaction_requested\n'
            'data: {"interaction_id":"i-5","kind":"question",'
            '"created_at":"2026-01-02T03:04:05Z",'
            '"expires_at":"2026-01-02T03:09:05Z"}\n\n';
        final events = await parse('$hidden$bare');
        expect((events[0] as InteractionRequestedEvent).rendersCard, isFalse);
        expect((events[1] as InteractionRequestedEvent).rendersCard, isTrue);
      });

      test('parses a resolved interaction with its response', () async {
        const raw =
            'id: 9\n'
            'event: interaction_resolved\n'
            'data: {"interaction_id":"i-1","kind":"tool_approval",'
            '"outcome":"resolved","resolved_at":"2026-01-02T03:05:00Z",'
            '"response":{"decision":"allow_always"},"rule_id":"r-1"}\n\n';
        final event = (await parse(raw)).single as InteractionResolvedEvent;
        expect(event.interactionId, 'i-1');
        expect(event.kind, 'tool_approval');
        expect(event.outcome, InteractionOutcome.resolved);
        expect(event.resolvedAt, DateTime.utc(2026, 1, 2, 3, 5));
        expect(event.response, {'decision': 'allow_always'});
        expect(event.ruleId, 'r-1');
        expect(event.sequenceId, 9);
      });

      test('parses cancelled and expired without a response', () async {
        String frame(String outcome) =>
            'event: interaction_resolved\n'
            'data: {"interaction_id":"i-1","kind":"question",'
            '"outcome":"$outcome","resolved_at":"2026-01-02T03:05:00Z"}\n\n';
        final events = await parse('${frame('cancelled')}${frame('expired')}');
        expect(events.cast<InteractionResolvedEvent>().map((e) => e.outcome), [
          InteractionOutcome.cancelled,
          InteractionOutcome.expired,
        ]);
        expect(
          events.cast<InteractionResolvedEvent>().every(
            (e) => e.response == null && e.ruleId == null,
          ),
          isTrue,
        );
      });

      test('drops malformed JSON and frames missing required fields', () async {
        const raw =
            'event: interaction_requested\ndata: {not json\n\n'
            'event: interaction_requested\ndata: {"kind":"question"}\n\n'
            'event: interaction_resolved\ndata: {not json\n\n'
            'event: interaction_resolved\n'
            'data: {"interaction_id":"i-1","kind":"question",'
            '"outcome":"sideways","resolved_at":"2026-01-02T03:05:00Z"}\n\n'
            'event: token\ndata: still here\n\n';
        final events = await parse(raw);
        expect(events.single, isA<TokenEvent>());
      });

      test('still ignores unknown event names', () async {
        expect(await parse('event: future_event\ndata: {}\n\n'), isEmpty);
      });
    });

    // -----------------------------------------------------------------------
    // Empty stream
    // -----------------------------------------------------------------------

    test('empty stream yields no events', () async {
      final events = await parseSseByteStream(const Stream<Uint8List>.empty())
          .toList();
      expect(events, isEmpty);
    });
  });
}
