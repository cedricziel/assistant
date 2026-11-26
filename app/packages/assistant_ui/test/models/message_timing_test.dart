import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

MessageTiming _timing({
  int? ttft,
  int total = 1000,
  int? tokens,
  int? decode,
}) => MessageTiming(
  timeToFirstToken: ttft == null ? null : Duration(milliseconds: ttft),
  total: Duration(milliseconds: total),
  outputTokens: tokens,
  decode: decode == null ? null : Duration(milliseconds: decode),
);

void main() {
  group('MessageTiming.fromJson', () {
    test('reads every field', () {
      final t = MessageTiming.fromJson(const {
        'time_to_first_token_ms': 800,
        'total_ms': 12400,
        'output_tokens': 420,
        'decode_ms': 10000,
      });

      expect(t.timeToFirstToken, const Duration(milliseconds: 800));
      expect(t.total, const Duration(milliseconds: 12400));
      expect(t.outputTokens, 420);
      expect(t.decode, const Duration(seconds: 10));
    });

    test('tolerates absent optional fields', () {
      final t = MessageTiming.fromJson(const {'total_ms': 65000});

      expect(t.total, const Duration(milliseconds: 65000));
      expect(t.timeToFirstToken, isNull);
      expect(t.outputTokens, isNull);
      expect(t.decode, isNull);
    });

    test('ignores unknown keys', () {
      final t = MessageTiming.fromJson(const {
        'total_ms': 500,
        'future_field': {'nested': true},
      });

      expect(t.total, const Duration(milliseconds: 500));
    });

    test('rejects a payload without total_ms', () {
      expect(
        () => MessageTiming.fromJson(const {'decode_ms': 10}),
        throwsFormatException,
      );
    });
  });

  group('tokensPerSecond', () {
    test('is output tokens over decode seconds, rounded', () {
      expect(_timing(tokens: 420, decode: 10000).tokensPerSecond, 42);
      expect(_timing(tokens: 100, decode: 3000).tokensPerSecond, 33);
    });

    test('is null without output tokens', () {
      expect(_timing(decode: 10000).tokensPerSecond, isNull);
    });

    test('is null without decode time', () {
      expect(_timing(tokens: 420).tokensPerSecond, isNull);
    });

    test('is null below 250 ms of decode time and present at 250 ms', () {
      expect(_timing(tokens: 5, decode: 249).tokensPerSecond, isNull);
      expect(_timing(tokens: 5, decode: 250).tokensPerSecond, 20);
    });
  });

  group('formatMessageTiming', () {
    test('joins first token, total and tokens per second', () {
      final t = _timing(ttft: 800, total: 12400, tokens: 420, decode: 10000);

      expect(
        formatMessageTiming(t),
        'First token 0.8 s · Total 12.4 s · 42 tok/s',
      );
    });

    test('shows minutes and seconds from 60 s', () {
      expect(formatMessageTiming(_timing(total: 65000)), 'Total 1 m 5 s');
      expect(formatMessageTiming(_timing(total: 120000)), 'Total 2 m 0 s');
    });

    test('shows sub-10 s durations with one decimal', () {
      expect(formatMessageTiming(_timing(total: 3000)), 'Total 3.0 s');
    });

    test('rolls a duration that rounds to 60 s over into minutes', () {
      expect(formatMessageTiming(_timing(total: 59960)), 'Total 1 m 0 s');
    });

    test('omits missing parts', () {
      expect(
        formatMessageTiming(_timing(ttft: 800)),
        'First token 0.8 s · '
        'Total 1.0 s',
      );
      expect(formatMessageTiming(_timing(total: 2000)), 'Total 2.0 s');
    });

    test('omits tokens per second when decode is too short', () {
      final t = _timing(total: 2000, tokens: 3, decode: 100);

      expect(formatMessageTiming(t), 'Total 2.0 s');
    });
  });

  group('describeMessageTiming', () {
    test('spells the parts out for a screen reader', () {
      final t = _timing(ttft: 800, total: 12400, tokens: 420, decode: 10000);

      expect(
        describeMessageTiming(t),
        'Response timing: first token 0.8 seconds, total 12.4 seconds, '
        '42 tokens per second',
      );
    });

    test('spells out minutes and omits missing parts', () {
      expect(
        describeMessageTiming(_timing(total: 65000)),
        'Response timing: total 1 minute 5 seconds',
      );
    });
  });

  group('ChatMessage.timing', () {
    test('is null by default and mutable like its siblings', () {
      final m = ChatMessage(id: 'a', role: 'assistant', content: 'hi');
      expect(m.timing, isNull);

      final t = _timing(total: 900);
      m.timing = t;
      expect(m.timing, same(t));
    });

    test('survives copyWith', () {
      final t = _timing(total: 900);
      final m = ChatMessage(id: 'a', role: 'assistant', content: 'hi')
        ..timing = t;

      expect(m.copyWith(content: 'bye').timing, same(t));
    });
  });
}
