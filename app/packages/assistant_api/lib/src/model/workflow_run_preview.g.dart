// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_run_preview.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowRunPreview extends WorkflowRunPreview {
  @override
  final int maxSteps;
  @override
  final int maxVisitsPerNode;
  @override
  final String message;
  @override
  final String runId;
  @override
  final DateTime startedAt;
  @override
  final String status;
  @override
  final String workflowId;

  factory _$WorkflowRunPreview(
          [void Function(WorkflowRunPreviewBuilder)? updates]) =>
      (WorkflowRunPreviewBuilder()..update(updates))._build();

  _$WorkflowRunPreview._(
      {required this.maxSteps,
      required this.maxVisitsPerNode,
      required this.message,
      required this.runId,
      required this.startedAt,
      required this.status,
      required this.workflowId})
      : super._();
  @override
  WorkflowRunPreview rebuild(
          void Function(WorkflowRunPreviewBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowRunPreviewBuilder toBuilder() =>
      WorkflowRunPreviewBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowRunPreview &&
        maxSteps == other.maxSteps &&
        maxVisitsPerNode == other.maxVisitsPerNode &&
        message == other.message &&
        runId == other.runId &&
        startedAt == other.startedAt &&
        status == other.status &&
        workflowId == other.workflowId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, maxSteps.hashCode);
    _$hash = $jc(_$hash, maxVisitsPerNode.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, runId.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, workflowId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowRunPreview')
          ..add('maxSteps', maxSteps)
          ..add('maxVisitsPerNode', maxVisitsPerNode)
          ..add('message', message)
          ..add('runId', runId)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('workflowId', workflowId))
        .toString();
  }
}

class WorkflowRunPreviewBuilder
    implements Builder<WorkflowRunPreview, WorkflowRunPreviewBuilder> {
  _$WorkflowRunPreview? _$v;

  int? _maxSteps;
  int? get maxSteps => _$this._maxSteps;
  set maxSteps(int? maxSteps) => _$this._maxSteps = maxSteps;

  int? _maxVisitsPerNode;
  int? get maxVisitsPerNode => _$this._maxVisitsPerNode;
  set maxVisitsPerNode(int? maxVisitsPerNode) =>
      _$this._maxVisitsPerNode = maxVisitsPerNode;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _runId;
  String? get runId => _$this._runId;
  set runId(String? runId) => _$this._runId = runId;

  DateTime? _startedAt;
  DateTime? get startedAt => _$this._startedAt;
  set startedAt(DateTime? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _workflowId;
  String? get workflowId => _$this._workflowId;
  set workflowId(String? workflowId) => _$this._workflowId = workflowId;

  WorkflowRunPreviewBuilder() {
    WorkflowRunPreview._defaults(this);
  }

  WorkflowRunPreviewBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _maxSteps = $v.maxSteps;
      _maxVisitsPerNode = $v.maxVisitsPerNode;
      _message = $v.message;
      _runId = $v.runId;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _workflowId = $v.workflowId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowRunPreview other) {
    _$v = other as _$WorkflowRunPreview;
  }

  @override
  void update(void Function(WorkflowRunPreviewBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowRunPreview build() => _build();

  _$WorkflowRunPreview _build() {
    final _$result = _$v ??
        _$WorkflowRunPreview._(
          maxSteps: BuiltValueNullFieldError.checkNotNull(
              maxSteps, r'WorkflowRunPreview', 'maxSteps'),
          maxVisitsPerNode: BuiltValueNullFieldError.checkNotNull(
              maxVisitsPerNode, r'WorkflowRunPreview', 'maxVisitsPerNode'),
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'WorkflowRunPreview', 'message'),
          runId: BuiltValueNullFieldError.checkNotNull(
              runId, r'WorkflowRunPreview', 'runId'),
          startedAt: BuiltValueNullFieldError.checkNotNull(
              startedAt, r'WorkflowRunPreview', 'startedAt'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'WorkflowRunPreview', 'status'),
          workflowId: BuiltValueNullFieldError.checkNotNull(
              workflowId, r'WorkflowRunPreview', 'workflowId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
