// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowDetail extends WorkflowDetail {
  @override
  final bool active;
  @override
  final DateTime createdAt;
  @override
  final String description;
  @override
  final JsonObject? graph;
  @override
  final String id;
  @override
  final String name;
  @override
  final DateTime updatedAt;

  factory _$WorkflowDetail([void Function(WorkflowDetailBuilder)? updates]) =>
      (WorkflowDetailBuilder()..update(updates))._build();

  _$WorkflowDetail._(
      {required this.active,
      required this.createdAt,
      required this.description,
      this.graph,
      required this.id,
      required this.name,
      required this.updatedAt})
      : super._();
  @override
  WorkflowDetail rebuild(void Function(WorkflowDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowDetailBuilder toBuilder() => WorkflowDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowDetail &&
        active == other.active &&
        createdAt == other.createdAt &&
        description == other.description &&
        graph == other.graph &&
        id == other.id &&
        name == other.name &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, graph.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowDetail')
          ..add('active', active)
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('graph', graph)
          ..add('id', id)
          ..add('name', name)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class WorkflowDetailBuilder
    implements Builder<WorkflowDetail, WorkflowDetailBuilder> {
  _$WorkflowDetail? _$v;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  JsonObject? _graph;
  JsonObject? get graph => _$this._graph;
  set graph(JsonObject? graph) => _$this._graph = graph;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  WorkflowDetailBuilder() {
    WorkflowDetail._defaults(this);
  }

  WorkflowDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _createdAt = $v.createdAt;
      _description = $v.description;
      _graph = $v.graph;
      _id = $v.id;
      _name = $v.name;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowDetail other) {
    _$v = other as _$WorkflowDetail;
  }

  @override
  void update(void Function(WorkflowDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowDetail build() => _build();

  _$WorkflowDetail _build() {
    final _$result = _$v ??
        _$WorkflowDetail._(
          active: BuiltValueNullFieldError.checkNotNull(
              active, r'WorkflowDetail', 'active'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'WorkflowDetail', 'createdAt'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'WorkflowDetail', 'description'),
          graph: graph,
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'WorkflowDetail', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'WorkflowDetail', 'name'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'WorkflowDetail', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
