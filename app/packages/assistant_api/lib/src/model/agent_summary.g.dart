// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentSummary extends AgentSummary {
  @override
  final String description;
  @override
  final String id;
  @override
  final int interfaceCount;
  @override
  final bool isDefault;
  @override
  final String name;
  @override
  final int skillCount;
  @override
  final String version;

  factory _$AgentSummary([void Function(AgentSummaryBuilder)? updates]) =>
      (AgentSummaryBuilder()..update(updates))._build();

  _$AgentSummary._(
      {required this.description,
      required this.id,
      required this.interfaceCount,
      required this.isDefault,
      required this.name,
      required this.skillCount,
      required this.version})
      : super._();
  @override
  AgentSummary rebuild(void Function(AgentSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentSummaryBuilder toBuilder() => AgentSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentSummary &&
        description == other.description &&
        id == other.id &&
        interfaceCount == other.interfaceCount &&
        isDefault == other.isDefault &&
        name == other.name &&
        skillCount == other.skillCount &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, interfaceCount.hashCode);
    _$hash = $jc(_$hash, isDefault.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, skillCount.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentSummary')
          ..add('description', description)
          ..add('id', id)
          ..add('interfaceCount', interfaceCount)
          ..add('isDefault', isDefault)
          ..add('name', name)
          ..add('skillCount', skillCount)
          ..add('version', version))
        .toString();
  }
}

class AgentSummaryBuilder
    implements Builder<AgentSummary, AgentSummaryBuilder> {
  _$AgentSummary? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  int? _interfaceCount;
  int? get interfaceCount => _$this._interfaceCount;
  set interfaceCount(int? interfaceCount) =>
      _$this._interfaceCount = interfaceCount;

  bool? _isDefault;
  bool? get isDefault => _$this._isDefault;
  set isDefault(bool? isDefault) => _$this._isDefault = isDefault;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  int? _skillCount;
  int? get skillCount => _$this._skillCount;
  set skillCount(int? skillCount) => _$this._skillCount = skillCount;

  String? _version;
  String? get version => _$this._version;
  set version(String? version) => _$this._version = version;

  AgentSummaryBuilder() {
    AgentSummary._defaults(this);
  }

  AgentSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _id = $v.id;
      _interfaceCount = $v.interfaceCount;
      _isDefault = $v.isDefault;
      _name = $v.name;
      _skillCount = $v.skillCount;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentSummary other) {
    _$v = other as _$AgentSummary;
  }

  @override
  void update(void Function(AgentSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentSummary build() => _build();

  _$AgentSummary _build() {
    final _$result = _$v ??
        _$AgentSummary._(
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'AgentSummary', 'description'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'AgentSummary', 'id'),
          interfaceCount: BuiltValueNullFieldError.checkNotNull(
              interfaceCount, r'AgentSummary', 'interfaceCount'),
          isDefault: BuiltValueNullFieldError.checkNotNull(
              isDefault, r'AgentSummary', 'isDefault'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'AgentSummary', 'name'),
          skillCount: BuiltValueNullFieldError.checkNotNull(
              skillCount, r'AgentSummary', 'skillCount'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'AgentSummary', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
