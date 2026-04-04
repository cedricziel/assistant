// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'stream_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$StreamResponse extends StreamResponse {
  @override
  final TaskArtifactUpdateEvent? artifactUpdate;
  @override
  final Message? message;
  @override
  final TaskStatusUpdateEvent? statusUpdate;
  @override
  final Task? task;

  factory _$StreamResponse([void Function(StreamResponseBuilder)? updates]) =>
      (StreamResponseBuilder()..update(updates))._build();

  _$StreamResponse._(
      {this.artifactUpdate, this.message, this.statusUpdate, this.task})
      : super._();
  @override
  StreamResponse rebuild(void Function(StreamResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  StreamResponseBuilder toBuilder() => StreamResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is StreamResponse &&
        artifactUpdate == other.artifactUpdate &&
        message == other.message &&
        statusUpdate == other.statusUpdate &&
        task == other.task;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, artifactUpdate.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, statusUpdate.hashCode);
    _$hash = $jc(_$hash, task.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'StreamResponse')
          ..add('artifactUpdate', artifactUpdate)
          ..add('message', message)
          ..add('statusUpdate', statusUpdate)
          ..add('task', task))
        .toString();
  }
}

class StreamResponseBuilder
    implements Builder<StreamResponse, StreamResponseBuilder> {
  _$StreamResponse? _$v;

  TaskArtifactUpdateEventBuilder? _artifactUpdate;
  TaskArtifactUpdateEventBuilder get artifactUpdate =>
      _$this._artifactUpdate ??= TaskArtifactUpdateEventBuilder();
  set artifactUpdate(TaskArtifactUpdateEventBuilder? artifactUpdate) =>
      _$this._artifactUpdate = artifactUpdate;

  MessageBuilder? _message;
  MessageBuilder get message => _$this._message ??= MessageBuilder();
  set message(MessageBuilder? message) => _$this._message = message;

  TaskStatusUpdateEventBuilder? _statusUpdate;
  TaskStatusUpdateEventBuilder get statusUpdate =>
      _$this._statusUpdate ??= TaskStatusUpdateEventBuilder();
  set statusUpdate(TaskStatusUpdateEventBuilder? statusUpdate) =>
      _$this._statusUpdate = statusUpdate;

  TaskBuilder? _task;
  TaskBuilder get task => _$this._task ??= TaskBuilder();
  set task(TaskBuilder? task) => _$this._task = task;

  StreamResponseBuilder() {
    StreamResponse._defaults(this);
  }

  StreamResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _artifactUpdate = $v.artifactUpdate?.toBuilder();
      _message = $v.message?.toBuilder();
      _statusUpdate = $v.statusUpdate?.toBuilder();
      _task = $v.task?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(StreamResponse other) {
    _$v = other as _$StreamResponse;
  }

  @override
  void update(void Function(StreamResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  StreamResponse build() => _build();

  _$StreamResponse _build() {
    _$StreamResponse _$result;
    try {
      _$result = _$v ??
          _$StreamResponse._(
            artifactUpdate: _artifactUpdate?.build(),
            message: _message?.build(),
            statusUpdate: _statusUpdate?.build(),
            task: _task?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'artifactUpdate';
        _artifactUpdate?.build();
        _$failedField = 'message';
        _message?.build();
        _$failedField = 'statusUpdate';
        _statusUpdate?.build();
        _$failedField = 'task';
        _task?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'StreamResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
