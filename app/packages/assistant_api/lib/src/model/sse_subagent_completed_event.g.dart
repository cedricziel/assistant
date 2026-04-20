// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_completed_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentCompletedEvent extends SseSubagentCompletedEvent {
  @override
  final String agentId;
  @override
  final String status;
  @override
  final String? summary;

  factory _$SseSubagentCompletedEvent(
          [void Function(SseSubagentCompletedEventBuilder)? updates]) =>
      (SseSubagentCompletedEventBuilder()..update(updates))._build();

  _$SseSubagentCompletedEvent._(
      {required this.agentId, required this.status, this.summary})
      : super._();
  @override
  SseSubagentCompletedEvent rebuild(
          void Function(SseSubagentCompletedEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentCompletedEventBuilder toBuilder() =>
      SseSubagentCompletedEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentCompletedEvent &&
        agentId == other.agentId &&
        status == other.status &&
        summary == other.summary;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, summary.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseSubagentCompletedEvent')
          ..add('agentId', agentId)
          ..add('status', status)
          ..add('summary', summary))
        .toString();
  }
}

class SseSubagentCompletedEventBuilder
    implements
        Builder<SseSubagentCompletedEvent, SseSubagentCompletedEventBuilder> {
  _$SseSubagentCompletedEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _summary;
  String? get summary => _$this._summary;
  set summary(String? summary) => _$this._summary = summary;

  SseSubagentCompletedEventBuilder() {
    SseSubagentCompletedEvent._defaults(this);
  }

  SseSubagentCompletedEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _status = $v.status;
      _summary = $v.summary;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentCompletedEvent other) {
    _$v = other as _$SseSubagentCompletedEvent;
  }

  @override
  void update(void Function(SseSubagentCompletedEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentCompletedEvent build() => _build();

  _$SseSubagentCompletedEvent _build() {
    final _$result = _$v ??
        _$SseSubagentCompletedEvent._(
          agentId: BuiltValueNullFieldError.checkNotNull(
              agentId, r'SseSubagentCompletedEvent', 'agentId'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'SseSubagentCompletedEvent', 'status'),
          summary: summary,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
