// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_status_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentStatusEvent extends SseSubagentStatusEvent {
  @override
  final String agentId;
  @override
  final SseStatusEvent data;

  factory _$SseSubagentStatusEvent(
          [void Function(SseSubagentStatusEventBuilder)? updates]) =>
      (SseSubagentStatusEventBuilder()..update(updates))._build();

  _$SseSubagentStatusEvent._({required this.agentId, required this.data})
      : super._();
  @override
  SseSubagentStatusEvent rebuild(
          void Function(SseSubagentStatusEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentStatusEventBuilder toBuilder() =>
      SseSubagentStatusEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentStatusEvent &&
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
    return (newBuiltValueToStringHelper(r'SseSubagentStatusEvent')
          ..add('agentId', agentId)
          ..add('data', data))
        .toString();
  }
}

class SseSubagentStatusEventBuilder
    implements Builder<SseSubagentStatusEvent, SseSubagentStatusEventBuilder> {
  _$SseSubagentStatusEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  SseStatusEventBuilder? _data;
  SseStatusEventBuilder get data => _$this._data ??= SseStatusEventBuilder();
  set data(SseStatusEventBuilder? data) => _$this._data = data;

  SseSubagentStatusEventBuilder() {
    SseSubagentStatusEvent._defaults(this);
  }

  SseSubagentStatusEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _data = $v.data.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentStatusEvent other) {
    _$v = other as _$SseSubagentStatusEvent;
  }

  @override
  void update(void Function(SseSubagentStatusEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentStatusEvent build() => _build();

  _$SseSubagentStatusEvent _build() {
    _$SseSubagentStatusEvent _$result;
    try {
      _$result = _$v ??
          _$SseSubagentStatusEvent._(
            agentId: BuiltValueNullFieldError.checkNotNull(
                agentId, r'SseSubagentStatusEvent', 'agentId'),
            data: data.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'data';
        data.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseSubagentStatusEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
