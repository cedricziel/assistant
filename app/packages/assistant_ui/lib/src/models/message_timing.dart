// -- Message timing ----------------------------------------------------------

/// Shortest decode time for which a tokens-per-second figure is meaningful.
const _minDecodeForRate = Duration(milliseconds: 250);

/// How long the server took to produce an assistant reply.
///
/// Mirrors the `timing` object of the `done` frame and of history messages.
/// Only [total] is always known; the rest is absent when the turn had no token
/// sink or the provider reported no usage.
class MessageTiming {
  const MessageTiming({
    this.timeToFirstToken,
    required this.total,
    this.outputTokens,
    this.decode,
  });

  /// Reads the wire format, ignoring unknown keys.
  ///
  /// Throws [FormatException] when `total_ms` is missing or not a number, so a
  /// caller can drop a malformed object without losing the surrounding event.
  factory MessageTiming.fromJson(Map<String, Object?> json) {
    final total = _millis(json['total_ms']);
    if (total == null) {
      throw const FormatException('timing is missing total_ms');
    }
    final tokens = json['output_tokens'];
    return MessageTiming(
      timeToFirstToken: _millis(json['time_to_first_token_ms']),
      total: total,
      outputTokens: tokens is num ? tokens.toInt() : null,
      decode: _millis(json['decode_ms']),
    );
  }

  /// From turn start to the first visible answer token.
  final Duration? timeToFirstToken;

  /// From turn start to just before the final answer was stored.
  final Duration total;

  /// Tokens produced across every model call of the turn.
  final int? outputTokens;

  /// Time spent generating, excluding tool round-trips.
  final Duration? decode;

  /// Generation speed, or null when it cannot be stated honestly: tokens or
  /// decode time are unknown, or decode time is under 250 ms.
  int? get tokensPerSecond {
    final tokens = outputTokens;
    final decode = this.decode;
    if (tokens == null || decode == null || decode < _minDecodeForRate) {
      return null;
    }
    return (tokens / (decode.inMicroseconds / Duration.microsecondsPerSecond))
        .round();
  }
}

Duration? _millis(Object? value) =>
    value is num ? Duration(milliseconds: value.round()) : null;

// -- Formatting --------------------------------------------------------------

/// Tooltip text, for example `First token 0.8 s · Total 12.4 s · 42 tok/s`.
///
/// Missing parts are left out.
String formatMessageTiming(MessageTiming timing) {
  final rate = timing.tokensPerSecond;
  return [
    if (timing.timeToFirstToken case final ttft?) 'First token ${_short(ttft)}',
    'Total ${_short(timing.total)}',
    if (rate != null) '$rate tok/s',
  ].join(' · ');
}

/// Screen-reader sentence, for example `Response timing: first token 0.8
/// seconds, total 12.4 seconds, 42 tokens per second`.
String describeMessageTiming(MessageTiming timing) {
  final rate = timing.tokensPerSecond;
  final parts = [
    if (timing.timeToFirstToken case final ttft?)
      'first token ${_spoken(ttft)}',
    'total ${_spoken(timing.total)}',
    if (rate != null) '$rate tokens per second',
  ];
  return 'Response timing: ${parts.join(', ')}';
}

/// Below a minute: seconds with one decimal. From a minute: whole minutes and
/// seconds.
({int minutes, int seconds, double? tenths}) _split(Duration d) {
  final tenths = (d.inMilliseconds / 100).round();
  if (tenths < 600) return (minutes: 0, seconds: 0, tenths: tenths / 10);
  final seconds = (d.inMilliseconds / 1000).round();
  return (minutes: seconds ~/ 60, seconds: seconds % 60, tenths: null);
}

String _short(Duration d) {
  final p = _split(d);
  final tenths = p.tenths;
  if (tenths != null) return '${tenths.toStringAsFixed(1)} s';
  return '${p.minutes} m ${p.seconds} s';
}

String _spoken(Duration d) {
  final p = _split(d);
  final tenths = p.tenths;
  if (tenths != null) return '${tenths.toStringAsFixed(1)} seconds';
  final minutes = p.minutes == 1 ? 'minute' : 'minutes';
  final seconds = p.seconds == 1 ? 'second' : 'seconds';
  return '${p.minutes} $minutes ${p.seconds} $seconds';
}
