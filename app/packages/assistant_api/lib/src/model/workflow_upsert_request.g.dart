// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_upsert_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowUpsertRequest extends WorkflowUpsertRequest {
  @override
  final bool? active;
  @override
  final String? description;
  @override
  final JsonObject? graph;
  @override
  final String name;

  factory _$WorkflowUpsertRequest(
          [void Function(WorkflowUpsertRequestBuilder)? updates]) =>
      (WorkflowUpsertRequestBuilder()..update(updates))._build();

  _$WorkflowUpsertRequest._(
      {this.active, this.description, this.graph, required this.name})
      : super._();
  @override
  WorkflowUpsertRequest rebuild(
          void Function(WorkflowUpsertRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowUpsertRequestBuilder toBuilder() =>
      WorkflowUpsertRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowUpsertRequest &&
        active == other.active &&
        description == other.description &&
        graph == other.graph &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, graph.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowUpsertRequest')
          ..add('active', active)
          ..add('description', description)
          ..add('graph', graph)
          ..add('name', name))
        .toString();
  }
}

class WorkflowUpsertRequestBuilder
    implements Builder<WorkflowUpsertRequest, WorkflowUpsertRequestBuilder> {
  _$WorkflowUpsertRequest? _$v;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  JsonObject? _graph;
  JsonObject? get graph => _$this._graph;
  set graph(JsonObject? graph) => _$this._graph = graph;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  WorkflowUpsertRequestBuilder() {
    WorkflowUpsertRequest._defaults(this);
  }

  WorkflowUpsertRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _description = $v.description;
      _graph = $v.graph;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowUpsertRequest other) {
    _$v = other as _$WorkflowUpsertRequest;
  }

  @override
  void update(void Function(WorkflowUpsertRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowUpsertRequest build() => _build();

  _$WorkflowUpsertRequest _build() {
    final _$result = _$v ??
        _$WorkflowUpsertRequest._(
          active: active,
          description: description,
          graph: graph,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'WorkflowUpsertRequest', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
