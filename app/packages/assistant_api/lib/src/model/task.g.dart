// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$Task extends Task {
  @override
  final BuiltList<Artifact>? artifacts;
  @override
  final String contextId;
  @override
  final BuiltList<Message>? history;
  @override
  final String id;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final TaskStatus status;

  factory _$Task([void Function(TaskBuilder)? updates]) =>
      (TaskBuilder()..update(updates))._build();

  _$Task._(
      {this.artifacts,
      required this.contextId,
      this.history,
      required this.id,
      this.metadata,
      required this.status})
      : super._();
  @override
  Task rebuild(void Function(TaskBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TaskBuilder toBuilder() => TaskBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is Task &&
        artifacts == other.artifacts &&
        contextId == other.contextId &&
        history == other.history &&
        id == other.id &&
        metadata == other.metadata &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, artifacts.hashCode);
    _$hash = $jc(_$hash, contextId.hashCode);
    _$hash = $jc(_$hash, history.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'Task')
          ..add('artifacts', artifacts)
          ..add('contextId', contextId)
          ..add('history', history)
          ..add('id', id)
          ..add('metadata', metadata)
          ..add('status', status))
        .toString();
  }
}

class TaskBuilder implements Builder<Task, TaskBuilder> {
  _$Task? _$v;

  ListBuilder<Artifact>? _artifacts;
  ListBuilder<Artifact> get artifacts =>
      _$this._artifacts ??= ListBuilder<Artifact>();
  set artifacts(ListBuilder<Artifact>? artifacts) =>
      _$this._artifacts = artifacts;

  String? _contextId;
  String? get contextId => _$this._contextId;
  set contextId(String? contextId) => _$this._contextId = contextId;

  ListBuilder<Message>? _history;
  ListBuilder<Message> get history =>
      _$this._history ??= ListBuilder<Message>();
  set history(ListBuilder<Message>? history) => _$this._history = history;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  TaskStatusBuilder? _status;
  TaskStatusBuilder get status => _$this._status ??= TaskStatusBuilder();
  set status(TaskStatusBuilder? status) => _$this._status = status;

  TaskBuilder() {
    Task._defaults(this);
  }

  TaskBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _artifacts = $v.artifacts?.toBuilder();
      _contextId = $v.contextId;
      _history = $v.history?.toBuilder();
      _id = $v.id;
      _metadata = $v.metadata?.toBuilder();
      _status = $v.status.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(Task other) {
    _$v = other as _$Task;
  }

  @override
  void update(void Function(TaskBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  Task build() => _build();

  _$Task _build() {
    _$Task _$result;
    try {
      _$result = _$v ??
          _$Task._(
            artifacts: _artifacts?.build(),
            contextId: BuiltValueNullFieldError.checkNotNull(
                contextId, r'Task', 'contextId'),
            history: _history?.build(),
            id: BuiltValueNullFieldError.checkNotNull(id, r'Task', 'id'),
            metadata: _metadata?.build(),
            status: status.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'artifacts';
        _artifacts?.build();

        _$failedField = 'history';
        _history?.build();

        _$failedField = 'metadata';
        _metadata?.build();
        _$failedField = 'status';
        status.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(r'Task', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
