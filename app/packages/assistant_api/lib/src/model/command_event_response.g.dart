// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'command_event_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CommandEventResponse extends CommandEventResponse {
  @override
  final String? ackText;
  @override
  final String command;
  @override
  final DateTime createdAt;
  @override
  final String eventType;
  @override
  final String id;
  @override
  final JsonObject? payload;

  factory _$CommandEventResponse(
          [void Function(CommandEventResponseBuilder)? updates]) =>
      (CommandEventResponseBuilder()..update(updates))._build();

  _$CommandEventResponse._(
      {this.ackText,
      required this.command,
      required this.createdAt,
      required this.eventType,
      required this.id,
      this.payload})
      : super._();
  @override
  CommandEventResponse rebuild(
          void Function(CommandEventResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CommandEventResponseBuilder toBuilder() =>
      CommandEventResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CommandEventResponse &&
        ackText == other.ackText &&
        command == other.command &&
        createdAt == other.createdAt &&
        eventType == other.eventType &&
        id == other.id &&
        payload == other.payload;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, ackText.hashCode);
    _$hash = $jc(_$hash, command.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, eventType.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, payload.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CommandEventResponse')
          ..add('ackText', ackText)
          ..add('command', command)
          ..add('createdAt', createdAt)
          ..add('eventType', eventType)
          ..add('id', id)
          ..add('payload', payload))
        .toString();
  }
}

class CommandEventResponseBuilder
    implements Builder<CommandEventResponse, CommandEventResponseBuilder> {
  _$CommandEventResponse? _$v;

  String? _ackText;
  String? get ackText => _$this._ackText;
  set ackText(String? ackText) => _$this._ackText = ackText;

  String? _command;
  String? get command => _$this._command;
  set command(String? command) => _$this._command = command;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _eventType;
  String? get eventType => _$this._eventType;
  set eventType(String? eventType) => _$this._eventType = eventType;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  JsonObject? _payload;
  JsonObject? get payload => _$this._payload;
  set payload(JsonObject? payload) => _$this._payload = payload;

  CommandEventResponseBuilder() {
    CommandEventResponse._defaults(this);
  }

  CommandEventResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _ackText = $v.ackText;
      _command = $v.command;
      _createdAt = $v.createdAt;
      _eventType = $v.eventType;
      _id = $v.id;
      _payload = $v.payload;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CommandEventResponse other) {
    _$v = other as _$CommandEventResponse;
  }

  @override
  void update(void Function(CommandEventResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CommandEventResponse build() => _build();

  _$CommandEventResponse _build() {
    final _$result = _$v ??
        _$CommandEventResponse._(
          ackText: ackText,
          command: BuiltValueNullFieldError.checkNotNull(
              command, r'CommandEventResponse', 'command'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'CommandEventResponse', 'createdAt'),
          eventType: BuiltValueNullFieldError.checkNotNull(
              eventType, r'CommandEventResponse', 'eventType'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'CommandEventResponse', 'id'),
          payload: payload,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
