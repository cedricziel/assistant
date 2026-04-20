// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_thinking_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentThinkingEvent extends SseSubagentThinkingEvent {
  @override
  final String agentId;
  @override
  final SseThinkingEvent data;

  factory _$SseSubagentThinkingEvent(
          [void Function(SseSubagentThinkingEventBuilder)? updates]) =>
      (SseSubagentThinkingEventBuilder()..update(updates))._build();

  _$SseSubagentThinkingEvent._({required this.agentId, required this.data})
      : super._();
  @override
  SseSubagentThinkingEvent rebuild(
          void Function(SseSubagentThinkingEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentThinkingEventBuilder toBuilder() =>
      SseSubagentThinkingEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentThinkingEvent &&
        agentId == other.agentId &&
        data == other.data;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, data.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseSubagentThinkingEvent')
          ..add('agentId', agentId)
          ..add('data', data))
        .toString();
  }
}

class SseSubagentThinkingEventBuilder
    implements
        Builder<SseSubagentThinkingEvent, SseSubagentThinkingEventBuilder> {
  _$SseSubagentThinkingEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  SseThinkingEventBuilder? _data;
  SseThinkingEventBuilder get data =>
      _$this._data ??= SseThinkingEventBuilder();
  set data(SseThinkingEventBuilder? data) => _$this._data = data;

  SseSubagentThinkingEventBuilder() {
    SseSubagentThinkingEvent._defaults(this);
  }

  SseSubagentThinkingEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _data = $v.data.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentThinkingEvent other) {
    _$v = other as _$SseSubagentThinkingEvent;
  }

  @override
  void update(void Function(SseSubagentThinkingEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentThinkingEvent build() => _build();

  _$SseSubagentThinkingEvent _build() {
    _$SseSubagentThinkingEvent _$result;
    try {
      _$result = _$v ??
          _$SseSubagentThinkingEvent._(
            agentId: BuiltValueNullFieldError.checkNotNull(
                agentId, r'SseSubagentThinkingEvent', 'agentId'),
            data: data.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'data';
        data.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseSubagentThinkingEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
