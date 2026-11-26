// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_tool_result_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseToolResultEvent extends SseToolResultEvent {
  @override
  final JsonObject? arguments;
  @override
  final JsonObject? data;
  @override
  final String? result;
  @override
  final String status;
  @override
  final String? toolCallId;
  @override
  final String toolName;

  factory _$SseToolResultEvent(
          [void Function(SseToolResultEventBuilder)? updates]) =>
      (SseToolResultEventBuilder()..update(updates))._build();

  _$SseToolResultEvent._(
      {this.arguments,
      this.data,
      this.result,
      required this.status,
      this.toolCallId,
      required this.toolName})
      : super._();
  @override
  SseToolResultEvent rebuild(
          void Function(SseToolResultEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseToolResultEventBuilder toBuilder() =>
      SseToolResultEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseToolResultEvent &&
        arguments == other.arguments &&
        data == other.data &&
        result == other.result &&
        status == other.status &&
        toolCallId == other.toolCallId &&
        toolName == other.toolName;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, arguments.hashCode);
    _$hash = $jc(_$hash, data.hashCode);
    _$hash = $jc(_$hash, result.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, toolCallId.hashCode);
    _$hash = $jc(_$hash, toolName.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseToolResultEvent')
          ..add('arguments', arguments)
          ..add('data', data)
          ..add('result', result)
          ..add('status', status)
          ..add('toolCallId', toolCallId)
          ..add('toolName', toolName))
        .toString();
  }
}

class SseToolResultEventBuilder
    implements Builder<SseToolResultEvent, SseToolResultEventBuilder> {
  _$SseToolResultEvent? _$v;

  JsonObject? _arguments;
  JsonObject? get arguments => _$this._arguments;
  set arguments(JsonObject? arguments) => _$this._arguments = arguments;

  JsonObject? _data;
  JsonObject? get data => _$this._data;
  set data(JsonObject? data) => _$this._data = data;

  String? _result;
  String? get result => _$this._result;
  set result(String? result) => _$this._result = result;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _toolCallId;
  String? get toolCallId => _$this._toolCallId;
  set toolCallId(String? toolCallId) => _$this._toolCallId = toolCallId;

  String? _toolName;
  String? get toolName => _$this._toolName;
  set toolName(String? toolName) => _$this._toolName = toolName;

  SseToolResultEventBuilder() {
    SseToolResultEvent._defaults(this);
  }

  SseToolResultEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _arguments = $v.arguments;
      _data = $v.data;
      _result = $v.result;
      _status = $v.status;
      _toolCallId = $v.toolCallId;
      _toolName = $v.toolName;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseToolResultEvent other) {
    _$v = other as _$SseToolResultEvent;
  }

  @override
  void update(void Function(SseToolResultEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseToolResultEvent build() => _build();

  _$SseToolResultEvent _build() {
    final _$result = _$v ??
        _$SseToolResultEvent._(
          arguments: arguments,
          data: data,
          result: result,
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'SseToolResultEvent', 'status'),
          toolCallId: toolCallId,
          toolName: BuiltValueNullFieldError.checkNotNull(
              toolName, r'SseToolResultEvent', 'toolName'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
