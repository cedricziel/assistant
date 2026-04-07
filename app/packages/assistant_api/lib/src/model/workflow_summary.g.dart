// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowSummary extends WorkflowSummary {
  @override
  final int actionCount;
  @override
  final bool active;
  @override
  final DateTime createdAt;
  @override
  final String description;
  @override
  final int edgeCount;
  @override
  final String id;
  @override
  final int maxSteps;
  @override
  final int maxVisitsPerNode;
  @override
  final String name;
  @override
  final int nodeCount;
  @override
  final int triggerCount;
  @override
  final DateTime updatedAt;

  factory _$WorkflowSummary([void Function(WorkflowSummaryBuilder)? updates]) =>
      (WorkflowSummaryBuilder()..update(updates))._build();

  _$WorkflowSummary._(
      {required this.actionCount,
      required this.active,
      required this.createdAt,
      required this.description,
      required this.edgeCount,
      required this.id,
      required this.maxSteps,
      required this.maxVisitsPerNode,
      required this.name,
      required this.nodeCount,
      required this.triggerCount,
      required this.updatedAt})
      : super._();
  @override
  WorkflowSummary rebuild(void Function(WorkflowSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowSummaryBuilder toBuilder() => WorkflowSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowSummary &&
        actionCount == other.actionCount &&
        active == other.active &&
        createdAt == other.createdAt &&
        description == other.description &&
        edgeCount == other.edgeCount &&
        id == other.id &&
        maxSteps == other.maxSteps &&
        maxVisitsPerNode == other.maxVisitsPerNode &&
        name == other.name &&
        nodeCount == other.nodeCount &&
        triggerCount == other.triggerCount &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, actionCount.hashCode);
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, edgeCount.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, maxSteps.hashCode);
    _$hash = $jc(_$hash, maxVisitsPerNode.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, nodeCount.hashCode);
    _$hash = $jc(_$hash, triggerCount.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowSummary')
          ..add('actionCount', actionCount)
          ..add('active', active)
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('edgeCount', edgeCount)
          ..add('id', id)
          ..add('maxSteps', maxSteps)
          ..add('maxVisitsPerNode', maxVisitsPerNode)
          ..add('name', name)
          ..add('nodeCount', nodeCount)
          ..add('triggerCount', triggerCount)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class WorkflowSummaryBuilder
    implements Builder<WorkflowSummary, WorkflowSummaryBuilder> {
  _$WorkflowSummary? _$v;

  int? _actionCount;
  int? get actionCount => _$this._actionCount;
  set actionCount(int? actionCount) => _$this._actionCount = actionCount;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  int? _edgeCount;
  int? get edgeCount => _$this._edgeCount;
  set edgeCount(int? edgeCount) => _$this._edgeCount = edgeCount;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  int? _maxSteps;
  int? get maxSteps => _$this._maxSteps;
  set maxSteps(int? maxSteps) => _$this._maxSteps = maxSteps;

  int? _maxVisitsPerNode;
  int? get maxVisitsPerNode => _$this._maxVisitsPerNode;
  set maxVisitsPerNode(int? maxVisitsPerNode) =>
      _$this._maxVisitsPerNode = maxVisitsPerNode;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  int? _nodeCount;
  int? get nodeCount => _$this._nodeCount;
  set nodeCount(int? nodeCount) => _$this._nodeCount = nodeCount;

  int? _triggerCount;
  int? get triggerCount => _$this._triggerCount;
  set triggerCount(int? triggerCount) => _$this._triggerCount = triggerCount;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  WorkflowSummaryBuilder() {
    WorkflowSummary._defaults(this);
  }

  WorkflowSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _actionCount = $v.actionCount;
      _active = $v.active;
      _createdAt = $v.createdAt;
      _description = $v.description;
      _edgeCount = $v.edgeCount;
      _id = $v.id;
      _maxSteps = $v.maxSteps;
      _maxVisitsPerNode = $v.maxVisitsPerNode;
      _name = $v.name;
      _nodeCount = $v.nodeCount;
      _triggerCount = $v.triggerCount;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowSummary other) {
    _$v = other as _$WorkflowSummary;
  }

  @override
  void update(void Function(WorkflowSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowSummary build() => _build();

  _$WorkflowSummary _build() {
    final _$result = _$v ??
        _$WorkflowSummary._(
          actionCount: BuiltValueNullFieldError.checkNotNull(
              actionCount, r'WorkflowSummary', 'actionCount'),
          active: BuiltValueNullFieldError.checkNotNull(
              active, r'WorkflowSummary', 'active'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'WorkflowSummary', 'createdAt'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'WorkflowSummary', 'description'),
          edgeCount: BuiltValueNullFieldError.checkNotNull(
              edgeCount, r'WorkflowSummary', 'edgeCount'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'WorkflowSummary', 'id'),
          maxSteps: BuiltValueNullFieldError.checkNotNull(
              maxSteps, r'WorkflowSummary', 'maxSteps'),
          maxVisitsPerNode: BuiltValueNullFieldError.checkNotNull(
              maxVisitsPerNode, r'WorkflowSummary', 'maxVisitsPerNode'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'WorkflowSummary', 'name'),
          nodeCount: BuiltValueNullFieldError.checkNotNull(
              nodeCount, r'WorkflowSummary', 'nodeCount'),
          triggerCount: BuiltValueNullFieldError.checkNotNull(
              triggerCount, r'WorkflowSummary', 'triggerCount'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'WorkflowSummary', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
