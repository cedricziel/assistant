// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_timing_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageTimingSummary extends MessageTimingSummary {
  @override
  final int? decodeMs;
  @override
  final int? outputTokens;
  @override
  final int? timeToFirstTokenMs;
  @override
  final int totalMs;

  factory _$MessageTimingSummary(
          [void Function(MessageTimingSummaryBuilder)? updates]) =>
      (MessageTimingSummaryBuilder()..update(updates))._build();

  _$MessageTimingSummary._(
      {this.decodeMs,
      this.outputTokens,
      this.timeToFirstTokenMs,
      required this.totalMs})
      : super._();
  @override
  MessageTimingSummary rebuild(
          void Function(MessageTimingSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageTimingSummaryBuilder toBuilder() =>
      MessageTimingSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MessageTimingSummary &&
        decodeMs == other.decodeMs &&
        outputTokens == other.outputTokens &&
        timeToFirstTokenMs == other.timeToFirstTokenMs &&
        totalMs == other.totalMs;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, decodeMs.hashCode);
    _$hash = $jc(_$hash, outputTokens.hashCode);
    _$hash = $jc(_$hash, timeToFirstTokenMs.hashCode);
    _$hash = $jc(_$hash, totalMs.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageTimingSummary')
          ..add('decodeMs', decodeMs)
          ..add('outputTokens', outputTokens)
          ..add('timeToFirstTokenMs', timeToFirstTokenMs)
          ..add('totalMs', totalMs))
        .toString();
  }
}

class MessageTimingSummaryBuilder
    implements Builder<MessageTimingSummary, MessageTimingSummaryBuilder> {
  _$MessageTimingSummary? _$v;

  int? _decodeMs;
  int? get decodeMs => _$this._decodeMs;
  set decodeMs(int? decodeMs) => _$this._decodeMs = decodeMs;

  int? _outputTokens;
  int? get outputTokens => _$this._outputTokens;
  set outputTokens(int? outputTokens) => _$this._outputTokens = outputTokens;

  int? _timeToFirstTokenMs;
  int? get timeToFirstTokenMs => _$this._timeToFirstTokenMs;
  set timeToFirstTokenMs(int? timeToFirstTokenMs) =>
      _$this._timeToFirstTokenMs = timeToFirstTokenMs;

  int? _totalMs;
  int? get totalMs => _$this._totalMs;
  set totalMs(int? totalMs) => _$this._totalMs = totalMs;

  MessageTimingSummaryBuilder() {
    MessageTimingSummary._defaults(this);
  }

  MessageTimingSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _decodeMs = $v.decodeMs;
      _outputTokens = $v.outputTokens;
      _timeToFirstTokenMs = $v.timeToFirstTokenMs;
      _totalMs = $v.totalMs;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MessageTimingSummary other) {
    _$v = other as _$MessageTimingSummary;
  }

  @override
  void update(void Function(MessageTimingSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MessageTimingSummary build() => _build();

  _$MessageTimingSummary _build() {
    final _$result = _$v ??
        _$MessageTimingSummary._(
          decodeMs: decodeMs,
          outputTokens: outputTokens,
          timeToFirstTokenMs: timeToFirstTokenMs,
          totalMs: BuiltValueNullFieldError.checkNotNull(
              totalMs, r'MessageTimingSummary', 'totalMs'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
