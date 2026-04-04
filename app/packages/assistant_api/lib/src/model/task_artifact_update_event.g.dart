// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_artifact_update_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TaskArtifactUpdateEvent extends TaskArtifactUpdateEvent {
  @override
  final bool? append;
  @override
  final Artifact artifact;
  @override
  final String contextId;
  @override
  final bool? lastChunk;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final String taskId;

  factory _$TaskArtifactUpdateEvent(
          [void Function(TaskArtifactUpdateEventBuilder)? updates]) =>
      (TaskArtifactUpdateEventBuilder()..update(updates))._build();

  _$TaskArtifactUpdateEvent._(
      {this.append,
      required this.artifact,
      required this.contextId,
      this.lastChunk,
      this.metadata,
      required this.taskId})
      : super._();
  @override
  TaskArtifactUpdateEvent rebuild(
          void Function(TaskArtifactUpdateEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TaskArtifactUpdateEventBuilder toBuilder() =>
      TaskArtifactUpdateEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TaskArtifactUpdateEvent &&
        append == other.append &&
        artifact == other.artifact &&
        contextId == other.contextId &&
        lastChunk == other.lastChunk &&
        metadata == other.metadata &&
        taskId == other.taskId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, append.hashCode);
    _$hash = $jc(_$hash, artifact.hashCode);
    _$hash = $jc(_$hash, contextId.hashCode);
    _$hash = $jc(_$hash, lastChunk.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TaskArtifactUpdateEvent')
          ..add('append', append)
          ..add('artifact', artifact)
          ..add('contextId', contextId)
          ..add('lastChunk', lastChunk)
          ..add('metadata', metadata)
          ..add('taskId', taskId))
        .toString();
  }
}

class TaskArtifactUpdateEventBuilder
    implements
        Builder<TaskArtifactUpdateEvent, TaskArtifactUpdateEventBuilder> {
  _$TaskArtifactUpdateEvent? _$v;

  bool? _append;
  bool? get append => _$this._append;
  set append(bool? append) => _$this._append = append;

  ArtifactBuilder? _artifact;
  ArtifactBuilder get artifact => _$this._artifact ??= ArtifactBuilder();
  set artifact(ArtifactBuilder? artifact) => _$this._artifact = artifact;

  String? _contextId;
  String? get contextId => _$this._contextId;
  set contextId(String? contextId) => _$this._contextId = contextId;

  bool? _lastChunk;
  bool? get lastChunk => _$this._lastChunk;
  set lastChunk(bool? lastChunk) => _$this._lastChunk = lastChunk;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  TaskArtifactUpdateEventBuilder() {
    TaskArtifactUpdateEvent._defaults(this);
  }

  TaskArtifactUpdateEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _append = $v.append;
      _artifact = $v.artifact.toBuilder();
      _contextId = $v.contextId;
      _lastChunk = $v.lastChunk;
      _metadata = $v.metadata?.toBuilder();
      _taskId = $v.taskId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TaskArtifactUpdateEvent other) {
    _$v = other as _$TaskArtifactUpdateEvent;
  }

  @override
  void update(void Function(TaskArtifactUpdateEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TaskArtifactUpdateEvent build() => _build();

  _$TaskArtifactUpdateEvent _build() {
    _$TaskArtifactUpdateEvent _$result;
    try {
      _$result = _$v ??
          _$TaskArtifactUpdateEvent._(
            append: append,
            artifact: artifact.build(),
            contextId: BuiltValueNullFieldError.checkNotNull(
                contextId, r'TaskArtifactUpdateEvent', 'contextId'),
            lastChunk: lastChunk,
            metadata: _metadata?.build(),
            taskId: BuiltValueNullFieldError.checkNotNull(
                taskId, r'TaskArtifactUpdateEvent', 'taskId'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'artifact';
        artifact.build();

        _$failedField = 'metadata';
        _metadata?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TaskArtifactUpdateEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
