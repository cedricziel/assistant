// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'log_entry_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$LogEntryResponse extends LogEntryResponse {
  @override
  final String? conversationId;
  @override
  final JsonObject? fields;
  @override
  final String id;
  @override
  final String message;
  @override
  final String severity;
  @override
  final String target;
  @override
  final DateTime timestamp;
  @override
  final String? traceId;

  factory _$LogEntryResponse(
          [void Function(LogEntryResponseBuilder)? updates]) =>
      (LogEntryResponseBuilder()..update(updates))._build();

  _$LogEntryResponse._(
      {this.conversationId,
      this.fields,
      required this.id,
      required this.message,
      required this.severity,
      required this.target,
      required this.timestamp,
      this.traceId})
      : super._();
  @override
  LogEntryResponse rebuild(void Function(LogEntryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  LogEntryResponseBuilder toBuilder() =>
      LogEntryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is LogEntryResponse &&
        conversationId == other.conversationId &&
        fields == other.fields &&
        id == other.id &&
        message == other.message &&
        severity == other.severity &&
        target == other.target &&
        timestamp == other.timestamp &&
        traceId == other.traceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, conversationId.hashCode);
    _$hash = $jc(_$hash, fields.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, severity.hashCode);
    _$hash = $jc(_$hash, target.hashCode);
    _$hash = $jc(_$hash, timestamp.hashCode);
    _$hash = $jc(_$hash, traceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'LogEntryResponse')
          ..add('conversationId', conversationId)
          ..add('fields', fields)
          ..add('id', id)
          ..add('message', message)
          ..add('severity', severity)
          ..add('target', target)
          ..add('timestamp', timestamp)
          ..add('traceId', traceId))
        .toString();
  }
}

class LogEntryResponseBuilder
    implements Builder<LogEntryResponse, LogEntryResponseBuilder> {
  _$LogEntryResponse? _$v;

  String? _conversationId;
  String? get conversationId => _$this._conversationId;
  set conversationId(String? conversationId) =>
      _$this._conversationId = conversationId;

  JsonObject? _fields;
  JsonObject? get fields => _$this._fields;
  set fields(JsonObject? fields) => _$this._fields = fields;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _severity;
  String? get severity => _$this._severity;
  set severity(String? severity) => _$this._severity = severity;

  String? _target;
  String? get target => _$this._target;
  set target(String? target) => _$this._target = target;

  DateTime? _timestamp;
  DateTime? get timestamp => _$this._timestamp;
  set timestamp(DateTime? timestamp) => _$this._timestamp = timestamp;

  String? _traceId;
  String? get traceId => _$this._traceId;
  set traceId(String? traceId) => _$this._traceId = traceId;

  LogEntryResponseBuilder() {
    LogEntryResponse._defaults(this);
  }

  LogEntryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _conversationId = $v.conversationId;
      _fields = $v.fields;
      _id = $v.id;
      _message = $v.message;
      _severity = $v.severity;
      _target = $v.target;
      _timestamp = $v.timestamp;
      _traceId = $v.traceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(LogEntryResponse other) {
    _$v = other as _$LogEntryResponse;
  }

  @override
  void update(void Function(LogEntryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  LogEntryResponse build() => _build();

  _$LogEntryResponse _build() {
    final _$result = _$v ??
        _$LogEntryResponse._(
          conversationId: conversationId,
          fields: fields,
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'LogEntryResponse', 'id'),
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'LogEntryResponse', 'message'),
          severity: BuiltValueNullFieldError.checkNotNull(
              severity, r'LogEntryResponse', 'severity'),
          target: BuiltValueNullFieldError.checkNotNull(
              target, r'LogEntryResponse', 'target'),
          timestamp: BuiltValueNullFieldError.checkNotNull(
              timestamp, r'LogEntryResponse', 'timestamp'),
          traceId: traceId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
