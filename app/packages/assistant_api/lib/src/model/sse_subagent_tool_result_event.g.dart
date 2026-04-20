// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_tool_result_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentToolResultEvent extends SseSubagentToolResultEvent {
  @override
  final String agentId;
  @override
  final SseToolResultEvent data;

  factory _$SseSubagentToolResultEvent(
          [void Function(SseSubagentToolResultEventBuilder)? updates]) =>
      (SseSubagentToolResultEventBuilder()..update(updates))._build();

  _$SseSubagentToolResultEvent._({required this.agentId, required this.data})
      : super._();
  @override
  SseSubagentToolResultEvent rebuild(
          void Function(SseSubagentToolResultEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentToolResultEventBuilder toBuilder() =>
      SseSubagentToolResultEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentToolResultEvent &&
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
    return (newBuiltValueToStringHelper(r'SseSubagentToolResultEvent')
          ..add('agentId', agentId)
          ..add('data', data))
        .toString();
  }
}

class SseSubagentToolResultEventBuilder
    implements
        Builder<SseSubagentToolResultEvent, SseSubagentToolResultEventBuilder> {
  _$SseSubagentToolResultEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  SseToolResultEventBuilder? _data;
  SseToolResultEventBuilder get data =>
      _$this._data ??= SseToolResultEventBuilder();
  set data(SseToolResultEventBuilder? data) => _$this._data = data;

  SseSubagentToolResultEventBuilder() {
    SseSubagentToolResultEvent._defaults(this);
  }

  SseSubagentToolResultEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _data = $v.data.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentToolResultEvent other) {
    _$v = other as _$SseSubagentToolResultEvent;
  }

  @override
  void update(void Function(SseSubagentToolResultEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentToolResultEvent build() => _build();

  _$SseSubagentToolResultEvent _build() {
    _$SseSubagentToolResultEvent _$result;
    try {
      _$result = _$v ??
          _$SseSubagentToolResultEvent._(
            agentId: BuiltValueNullFieldError.checkNotNull(
                agentId, r'SseSubagentToolResultEvent', 'agentId'),
            data: data.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'data';
        data.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseSubagentToolResultEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
