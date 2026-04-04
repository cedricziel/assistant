// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'send_message_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SendMessageResponse extends SendMessageResponse {
  @override
  final Message? message;
  @override
  final Task? task;

  factory _$SendMessageResponse(
          [void Function(SendMessageResponseBuilder)? updates]) =>
      (SendMessageResponseBuilder()..update(updates))._build();

  _$SendMessageResponse._({this.message, this.task}) : super._();
  @override
  SendMessageResponse rebuild(
          void Function(SendMessageResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SendMessageResponseBuilder toBuilder() =>
      SendMessageResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SendMessageResponse &&
        message == other.message &&
        task == other.task;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, task.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SendMessageResponse')
          ..add('message', message)
          ..add('task', task))
        .toString();
  }
}

class SendMessageResponseBuilder
    implements Builder<SendMessageResponse, SendMessageResponseBuilder> {
  _$SendMessageResponse? _$v;

  MessageBuilder? _message;
  MessageBuilder get message => _$this._message ??= MessageBuilder();
  set message(MessageBuilder? message) => _$this._message = message;

  TaskBuilder? _task;
  TaskBuilder get task => _$this._task ??= TaskBuilder();
  set task(TaskBuilder? task) => _$this._task = task;

  SendMessageResponseBuilder() {
    SendMessageResponse._defaults(this);
  }

  SendMessageResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message?.toBuilder();
      _task = $v.task?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SendMessageResponse other) {
    _$v = other as _$SendMessageResponse;
  }

  @override
  void update(void Function(SendMessageResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SendMessageResponse build() => _build();

  _$SendMessageResponse _build() {
    _$SendMessageResponse _$result;
    try {
      _$result = _$v ??
          _$SendMessageResponse._(
            message: _message?.build(),
            task: _task?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'message';
        _message?.build();
        _$failedField = 'task';
        _task?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SendMessageResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
