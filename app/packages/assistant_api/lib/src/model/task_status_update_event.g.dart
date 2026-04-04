// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_status_update_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TaskStatusUpdateEvent extends TaskStatusUpdateEvent {
  @override
  final String contextId;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final TaskStatus status;
  @override
  final String taskId;

  factory _$TaskStatusUpdateEvent(
          [void Function(TaskStatusUpdateEventBuilder)? updates]) =>
      (TaskStatusUpdateEventBuilder()..update(updates))._build();

  _$TaskStatusUpdateEvent._(
      {required this.contextId,
      this.metadata,
      required this.status,
      required this.taskId})
      : super._();
  @override
  TaskStatusUpdateEvent rebuild(
          void Function(TaskStatusUpdateEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TaskStatusUpdateEventBuilder toBuilder() =>
      TaskStatusUpdateEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TaskStatusUpdateEvent &&
        contextId == other.contextId &&
        metadata == other.metadata &&
        status == other.status &&
        taskId == other.taskId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, contextId.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TaskStatusUpdateEvent')
          ..add('contextId', contextId)
          ..add('metadata', metadata)
          ..add('status', status)
          ..add('taskId', taskId))
        .toString();
  }
}

class TaskStatusUpdateEventBuilder
    implements Builder<TaskStatusUpdateEvent, TaskStatusUpdateEventBuilder> {
  _$TaskStatusUpdateEvent? _$v;

  String? _contextId;
  String? get contextId => _$this._contextId;
  set contextId(String? contextId) => _$this._contextId = contextId;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  TaskStatusBuilder? _status;
  TaskStatusBuilder get status => _$this._status ??= TaskStatusBuilder();
  set status(TaskStatusBuilder? status) => _$this._status = status;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  TaskStatusUpdateEventBuilder() {
    TaskStatusUpdateEvent._defaults(this);
  }

  TaskStatusUpdateEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _contextId = $v.contextId;
      _metadata = $v.metadata?.toBuilder();
      _status = $v.status.toBuilder();
      _taskId = $v.taskId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TaskStatusUpdateEvent other) {
    _$v = other as _$TaskStatusUpdateEvent;
  }

  @override
  void update(void Function(TaskStatusUpdateEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TaskStatusUpdateEvent build() => _build();

  _$TaskStatusUpdateEvent _build() {
    _$TaskStatusUpdateEvent _$result;
    try {
      _$result = _$v ??
          _$TaskStatusUpdateEvent._(
            contextId: BuiltValueNullFieldError.checkNotNull(
                contextId, r'TaskStatusUpdateEvent', 'contextId'),
            metadata: _metadata?.build(),
            status: status.build(),
            taskId: BuiltValueNullFieldError.checkNotNull(
                taskId, r'TaskStatusUpdateEvent', 'taskId'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'metadata';
        _metadata?.build();
        _$failedField = 'status';
        status.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TaskStatusUpdateEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
