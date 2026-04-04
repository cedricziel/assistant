// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_status.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TaskStatus extends TaskStatus {
  @override
  final Message? message;
  @override
  final TaskState state;
  @override
  final DateTime? timestamp;

  factory _$TaskStatus([void Function(TaskStatusBuilder)? updates]) =>
      (TaskStatusBuilder()..update(updates))._build();

  _$TaskStatus._({this.message, required this.state, this.timestamp})
      : super._();
  @override
  TaskStatus rebuild(void Function(TaskStatusBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TaskStatusBuilder toBuilder() => TaskStatusBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TaskStatus &&
        message == other.message &&
        state == other.state &&
        timestamp == other.timestamp;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, state.hashCode);
    _$hash = $jc(_$hash, timestamp.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TaskStatus')
          ..add('message', message)
          ..add('state', state)
          ..add('timestamp', timestamp))
        .toString();
  }
}

class TaskStatusBuilder implements Builder<TaskStatus, TaskStatusBuilder> {
  _$TaskStatus? _$v;

  MessageBuilder? _message;
  MessageBuilder get message => _$this._message ??= MessageBuilder();
  set message(MessageBuilder? message) => _$this._message = message;

  TaskState? _state;
  TaskState? get state => _$this._state;
  set state(TaskState? state) => _$this._state = state;

  DateTime? _timestamp;
  DateTime? get timestamp => _$this._timestamp;
  set timestamp(DateTime? timestamp) => _$this._timestamp = timestamp;

  TaskStatusBuilder() {
    TaskStatus._defaults(this);
  }

  TaskStatusBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message?.toBuilder();
      _state = $v.state;
      _timestamp = $v.timestamp;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TaskStatus other) {
    _$v = other as _$TaskStatus;
  }

  @override
  void update(void Function(TaskStatusBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TaskStatus build() => _build();

  _$TaskStatus _build() {
    _$TaskStatus _$result;
    try {
      _$result = _$v ??
          _$TaskStatus._(
            message: _message?.build(),
            state: BuiltValueNullFieldError.checkNotNull(
                state, r'TaskStatus', 'state'),
            timestamp: timestamp,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'message';
        _message?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TaskStatus', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
