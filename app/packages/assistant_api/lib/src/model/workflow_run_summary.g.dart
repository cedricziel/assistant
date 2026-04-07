// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_run_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowRunSummary extends WorkflowRunSummary {
  @override
  final String? errorMessage;
  @override
  final DateTime? finishedAt;
  @override
  final String id;
  @override
  final DateTime startedAt;
  @override
  final String status;
  @override
  final JsonObject? triggerPayload;
  @override
  final String triggerType;
  @override
  final String workflowId;

  factory _$WorkflowRunSummary(
          [void Function(WorkflowRunSummaryBuilder)? updates]) =>
      (WorkflowRunSummaryBuilder()..update(updates))._build();

  _$WorkflowRunSummary._(
      {this.errorMessage,
      this.finishedAt,
      required this.id,
      required this.startedAt,
      required this.status,
      this.triggerPayload,
      required this.triggerType,
      required this.workflowId})
      : super._();
  @override
  WorkflowRunSummary rebuild(
          void Function(WorkflowRunSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowRunSummaryBuilder toBuilder() =>
      WorkflowRunSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowRunSummary &&
        errorMessage == other.errorMessage &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        startedAt == other.startedAt &&
        status == other.status &&
        triggerPayload == other.triggerPayload &&
        triggerType == other.triggerType &&
        workflowId == other.workflowId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, errorMessage.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, triggerPayload.hashCode);
    _$hash = $jc(_$hash, triggerType.hashCode);
    _$hash = $jc(_$hash, workflowId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowRunSummary')
          ..add('errorMessage', errorMessage)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('triggerPayload', triggerPayload)
          ..add('triggerType', triggerType)
          ..add('workflowId', workflowId))
        .toString();
  }
}

class WorkflowRunSummaryBuilder
    implements Builder<WorkflowRunSummary, WorkflowRunSummaryBuilder> {
  _$WorkflowRunSummary? _$v;

  String? _errorMessage;
  String? get errorMessage => _$this._errorMessage;
  set errorMessage(String? errorMessage) => _$this._errorMessage = errorMessage;

  DateTime? _finishedAt;
  DateTime? get finishedAt => _$this._finishedAt;
  set finishedAt(DateTime? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  DateTime? _startedAt;
  DateTime? get startedAt => _$this._startedAt;
  set startedAt(DateTime? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  JsonObject? _triggerPayload;
  JsonObject? get triggerPayload => _$this._triggerPayload;
  set triggerPayload(JsonObject? triggerPayload) =>
      _$this._triggerPayload = triggerPayload;

  String? _triggerType;
  String? get triggerType => _$this._triggerType;
  set triggerType(String? triggerType) => _$this._triggerType = triggerType;

  String? _workflowId;
  String? get workflowId => _$this._workflowId;
  set workflowId(String? workflowId) => _$this._workflowId = workflowId;

  WorkflowRunSummaryBuilder() {
    WorkflowRunSummary._defaults(this);
  }

  WorkflowRunSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _errorMessage = $v.errorMessage;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _triggerPayload = $v.triggerPayload;
      _triggerType = $v.triggerType;
      _workflowId = $v.workflowId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowRunSummary other) {
    _$v = other as _$WorkflowRunSummary;
  }

  @override
  void update(void Function(WorkflowRunSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowRunSummary build() => _build();

  _$WorkflowRunSummary _build() {
    final _$result = _$v ??
        _$WorkflowRunSummary._(
          errorMessage: errorMessage,
          finishedAt: finishedAt,
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'WorkflowRunSummary', 'id'),
          startedAt: BuiltValueNullFieldError.checkNotNull(
              startedAt, r'WorkflowRunSummary', 'startedAt'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'WorkflowRunSummary', 'status'),
          triggerPayload: triggerPayload,
          triggerType: BuiltValueNullFieldError.checkNotNull(
              triggerType, r'WorkflowRunSummary', 'triggerType'),
          workflowId: BuiltValueNullFieldError.checkNotNull(
              workflowId, r'WorkflowRunSummary', 'workflowId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
