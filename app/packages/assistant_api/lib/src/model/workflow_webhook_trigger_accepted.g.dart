// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_webhook_trigger_accepted.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowWebhookTriggerAccepted extends WorkflowWebhookTriggerAccepted {
  @override
  final String runId;
  @override
  final String status;
  @override
  final String workflowId;

  factory _$WorkflowWebhookTriggerAccepted(
          [void Function(WorkflowWebhookTriggerAcceptedBuilder)? updates]) =>
      (WorkflowWebhookTriggerAcceptedBuilder()..update(updates))._build();

  _$WorkflowWebhookTriggerAccepted._(
      {required this.runId, required this.status, required this.workflowId})
      : super._();
  @override
  WorkflowWebhookTriggerAccepted rebuild(
          void Function(WorkflowWebhookTriggerAcceptedBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowWebhookTriggerAcceptedBuilder toBuilder() =>
      WorkflowWebhookTriggerAcceptedBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowWebhookTriggerAccepted &&
        runId == other.runId &&
        status == other.status &&
        workflowId == other.workflowId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, runId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, workflowId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowWebhookTriggerAccepted')
          ..add('runId', runId)
          ..add('status', status)
          ..add('workflowId', workflowId))
        .toString();
  }
}

class WorkflowWebhookTriggerAcceptedBuilder
    implements
        Builder<WorkflowWebhookTriggerAccepted,
            WorkflowWebhookTriggerAcceptedBuilder> {
  _$WorkflowWebhookTriggerAccepted? _$v;

  String? _runId;
  String? get runId => _$this._runId;
  set runId(String? runId) => _$this._runId = runId;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _workflowId;
  String? get workflowId => _$this._workflowId;
  set workflowId(String? workflowId) => _$this._workflowId = workflowId;

  WorkflowWebhookTriggerAcceptedBuilder() {
    WorkflowWebhookTriggerAccepted._defaults(this);
  }

  WorkflowWebhookTriggerAcceptedBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _runId = $v.runId;
      _status = $v.status;
      _workflowId = $v.workflowId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowWebhookTriggerAccepted other) {
    _$v = other as _$WorkflowWebhookTriggerAccepted;
  }

  @override
  void update(void Function(WorkflowWebhookTriggerAcceptedBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowWebhookTriggerAccepted build() => _build();

  _$WorkflowWebhookTriggerAccepted _build() {
    final _$result = _$v ??
        _$WorkflowWebhookTriggerAccepted._(
          runId: BuiltValueNullFieldError.checkNotNull(
              runId, r'WorkflowWebhookTriggerAccepted', 'runId'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'WorkflowWebhookTriggerAccepted', 'status'),
          workflowId: BuiltValueNullFieldError.checkNotNull(
              workflowId, r'WorkflowWebhookTriggerAccepted', 'workflowId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
