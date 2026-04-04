// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$Message extends Message {
  @override
  final String? contextId;
  @override
  final BuiltList<String>? extensions;
  @override
  final String messageId;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final BuiltList<ModelPart> parts;
  @override
  final BuiltList<String>? referenceTaskIds;
  @override
  final Role role;
  @override
  final String? taskId;

  factory _$Message([void Function(MessageBuilder)? updates]) =>
      (MessageBuilder()..update(updates))._build();

  _$Message._(
      {this.contextId,
      this.extensions,
      required this.messageId,
      this.metadata,
      required this.parts,
      this.referenceTaskIds,
      required this.role,
      this.taskId})
      : super._();
  @override
  Message rebuild(void Function(MessageBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageBuilder toBuilder() => MessageBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is Message &&
        contextId == other.contextId &&
        extensions == other.extensions &&
        messageId == other.messageId &&
        metadata == other.metadata &&
        parts == other.parts &&
        referenceTaskIds == other.referenceTaskIds &&
        role == other.role &&
        taskId == other.taskId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, contextId.hashCode);
    _$hash = $jc(_$hash, extensions.hashCode);
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, parts.hashCode);
    _$hash = $jc(_$hash, referenceTaskIds.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'Message')
          ..add('contextId', contextId)
          ..add('extensions', extensions)
          ..add('messageId', messageId)
          ..add('metadata', metadata)
          ..add('parts', parts)
          ..add('referenceTaskIds', referenceTaskIds)
          ..add('role', role)
          ..add('taskId', taskId))
        .toString();
  }
}

class MessageBuilder implements Builder<Message, MessageBuilder> {
  _$Message? _$v;

  String? _contextId;
  String? get contextId => _$this._contextId;
  set contextId(String? contextId) => _$this._contextId = contextId;

  ListBuilder<String>? _extensions;
  ListBuilder<String> get extensions =>
      _$this._extensions ??= ListBuilder<String>();
  set extensions(ListBuilder<String>? extensions) =>
      _$this._extensions = extensions;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  ListBuilder<ModelPart>? _parts;
  ListBuilder<ModelPart> get parts =>
      _$this._parts ??= ListBuilder<ModelPart>();
  set parts(ListBuilder<ModelPart>? parts) => _$this._parts = parts;

  ListBuilder<String>? _referenceTaskIds;
  ListBuilder<String> get referenceTaskIds =>
      _$this._referenceTaskIds ??= ListBuilder<String>();
  set referenceTaskIds(ListBuilder<String>? referenceTaskIds) =>
      _$this._referenceTaskIds = referenceTaskIds;

  Role? _role;
  Role? get role => _$this._role;
  set role(Role? role) => _$this._role = role;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  MessageBuilder() {
    Message._defaults(this);
  }

  MessageBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _contextId = $v.contextId;
      _extensions = $v.extensions?.toBuilder();
      _messageId = $v.messageId;
      _metadata = $v.metadata?.toBuilder();
      _parts = $v.parts.toBuilder();
      _referenceTaskIds = $v.referenceTaskIds?.toBuilder();
      _role = $v.role;
      _taskId = $v.taskId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(Message other) {
    _$v = other as _$Message;
  }

  @override
  void update(void Function(MessageBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  Message build() => _build();

  _$Message _build() {
    _$Message _$result;
    try {
      _$result = _$v ??
          _$Message._(
            contextId: contextId,
            extensions: _extensions?.build(),
            messageId: BuiltValueNullFieldError.checkNotNull(
                messageId, r'Message', 'messageId'),
            metadata: _metadata?.build(),
            parts: parts.build(),
            referenceTaskIds: _referenceTaskIds?.build(),
            role:
                BuiltValueNullFieldError.checkNotNull(role, r'Message', 'role'),
            taskId: taskId,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'extensions';
        _extensions?.build();

        _$failedField = 'metadata';
        _metadata?.build();
        _$failedField = 'parts';
        parts.build();
        _$failedField = 'referenceTaskIds';
        _referenceTaskIds?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'Message', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
