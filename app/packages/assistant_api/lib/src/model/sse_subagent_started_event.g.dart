// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_started_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentStartedEvent extends SseSubagentStartedEvent {
  @override
  final String agentId;
  @override
  final String task;

  factory _$SseSubagentStartedEvent(
          [void Function(SseSubagentStartedEventBuilder)? updates]) =>
      (SseSubagentStartedEventBuilder()..update(updates))._build();

  _$SseSubagentStartedEvent._({required this.agentId, required this.task})
      : super._();
  @override
  SseSubagentStartedEvent rebuild(
          void Function(SseSubagentStartedEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentStartedEventBuilder toBuilder() =>
      SseSubagentStartedEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentStartedEvent &&
        agentId == other.agentId &&
        task == other.task;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, task.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseSubagentStartedEvent')
          ..add('agentId', agentId)
          ..add('task', task))
        .toString();
  }
}

class SseSubagentStartedEventBuilder
    implements
        Builder<SseSubagentStartedEvent, SseSubagentStartedEventBuilder> {
  _$SseSubagentStartedEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _task;
  String? get task => _$this._task;
  set task(String? task) => _$this._task = task;

  SseSubagentStartedEventBuilder() {
    SseSubagentStartedEvent._defaults(this);
  }

  SseSubagentStartedEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _task = $v.task;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentStartedEvent other) {
    _$v = other as _$SseSubagentStartedEvent;
  }

  @override
  void update(void Function(SseSubagentStartedEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentStartedEvent build() => _build();

  _$SseSubagentStartedEvent _build() {
    final _$result = _$v ??
        _$SseSubagentStartedEvent._(
          agentId: BuiltValueNullFieldError.checkNotNull(
              agentId, r'SseSubagentStartedEvent', 'agentId'),
          task: BuiltValueNullFieldError.checkNotNull(
              task, r'SseSubagentStartedEvent', 'task'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
