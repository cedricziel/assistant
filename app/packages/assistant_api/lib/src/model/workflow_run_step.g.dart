// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_run_step.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowRunStep extends WorkflowRunStep {
  @override
  final String id;
  @override
  final String nodeId;
  @override
  final String nodeKind;
  @override
  final String? note;
  @override
  final DateTime occurredAt;
  @override
  final JsonObject? output;
  @override
  final int stepIndex;

  factory _$WorkflowRunStep([void Function(WorkflowRunStepBuilder)? updates]) =>
      (WorkflowRunStepBuilder()..update(updates))._build();

  _$WorkflowRunStep._(
      {required this.id,
      required this.nodeId,
      required this.nodeKind,
      this.note,
      required this.occurredAt,
      this.output,
      required this.stepIndex})
      : super._();
  @override
  WorkflowRunStep rebuild(void Function(WorkflowRunStepBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowRunStepBuilder toBuilder() => WorkflowRunStepBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowRunStep &&
        id == other.id &&
        nodeId == other.nodeId &&
        nodeKind == other.nodeKind &&
        note == other.note &&
        occurredAt == other.occurredAt &&
        output == other.output &&
        stepIndex == other.stepIndex;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, nodeKind.hashCode);
    _$hash = $jc(_$hash, note.hashCode);
    _$hash = $jc(_$hash, occurredAt.hashCode);
    _$hash = $jc(_$hash, output.hashCode);
    _$hash = $jc(_$hash, stepIndex.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowRunStep')
          ..add('id', id)
          ..add('nodeId', nodeId)
          ..add('nodeKind', nodeKind)
          ..add('note', note)
          ..add('occurredAt', occurredAt)
          ..add('output', output)
          ..add('stepIndex', stepIndex))
        .toString();
  }
}

class WorkflowRunStepBuilder
    implements Builder<WorkflowRunStep, WorkflowRunStepBuilder> {
  _$WorkflowRunStep? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _nodeKind;
  String? get nodeKind => _$this._nodeKind;
  set nodeKind(String? nodeKind) => _$this._nodeKind = nodeKind;

  String? _note;
  String? get note => _$this._note;
  set note(String? note) => _$this._note = note;

  DateTime? _occurredAt;
  DateTime? get occurredAt => _$this._occurredAt;
  set occurredAt(DateTime? occurredAt) => _$this._occurredAt = occurredAt;

  JsonObject? _output;
  JsonObject? get output => _$this._output;
  set output(JsonObject? output) => _$this._output = output;

  int? _stepIndex;
  int? get stepIndex => _$this._stepIndex;
  set stepIndex(int? stepIndex) => _$this._stepIndex = stepIndex;

  WorkflowRunStepBuilder() {
    WorkflowRunStep._defaults(this);
  }

  WorkflowRunStepBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _nodeId = $v.nodeId;
      _nodeKind = $v.nodeKind;
      _note = $v.note;
      _occurredAt = $v.occurredAt;
      _output = $v.output;
      _stepIndex = $v.stepIndex;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowRunStep other) {
    _$v = other as _$WorkflowRunStep;
  }

  @override
  void update(void Function(WorkflowRunStepBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowRunStep build() => _build();

  _$WorkflowRunStep _build() {
    final _$result = _$v ??
        _$WorkflowRunStep._(
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'WorkflowRunStep', 'id'),
          nodeId: BuiltValueNullFieldError.checkNotNull(
              nodeId, r'WorkflowRunStep', 'nodeId'),
          nodeKind: BuiltValueNullFieldError.checkNotNull(
              nodeKind, r'WorkflowRunStep', 'nodeKind'),
          note: note,
          occurredAt: BuiltValueNullFieldError.checkNotNull(
              occurredAt, r'WorkflowRunStep', 'occurredAt'),
          output: output,
          stepIndex: BuiltValueNullFieldError.checkNotNull(
              stepIndex, r'WorkflowRunStep', 'stepIndex'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
