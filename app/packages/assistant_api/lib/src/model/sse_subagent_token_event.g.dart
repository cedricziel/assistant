// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_subagent_token_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSubagentTokenEvent extends SseSubagentTokenEvent {
  @override
  final String agentId;
  @override
  final SseTokenEvent data;

  factory _$SseSubagentTokenEvent(
          [void Function(SseSubagentTokenEventBuilder)? updates]) =>
      (SseSubagentTokenEventBuilder()..update(updates))._build();

  _$SseSubagentTokenEvent._({required this.agentId, required this.data})
      : super._();
  @override
  SseSubagentTokenEvent rebuild(
          void Function(SseSubagentTokenEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSubagentTokenEventBuilder toBuilder() =>
      SseSubagentTokenEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSubagentTokenEvent &&
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
    return (newBuiltValueToStringHelper(r'SseSubagentTokenEvent')
          ..add('agentId', agentId)
          ..add('data', data))
        .toString();
  }
}

class SseSubagentTokenEventBuilder
    implements Builder<SseSubagentTokenEvent, SseSubagentTokenEventBuilder> {
  _$SseSubagentTokenEvent? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  SseTokenEventBuilder? _data;
  SseTokenEventBuilder get data => _$this._data ??= SseTokenEventBuilder();
  set data(SseTokenEventBuilder? data) => _$this._data = data;

  SseSubagentTokenEventBuilder() {
    SseSubagentTokenEvent._defaults(this);
  }

  SseSubagentTokenEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _data = $v.data.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSubagentTokenEvent other) {
    _$v = other as _$SseSubagentTokenEvent;
  }

  @override
  void update(void Function(SseSubagentTokenEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSubagentTokenEvent build() => _build();

  _$SseSubagentTokenEvent _build() {
    _$SseSubagentTokenEvent _$result;
    try {
      _$result = _$v ??
          _$SseSubagentTokenEvent._(
            agentId: BuiltValueNullFieldError.checkNotNull(
                agentId, r'SseSubagentTokenEvent', 'agentId'),
            data: data.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'data';
        data.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseSubagentTokenEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
