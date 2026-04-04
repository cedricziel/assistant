// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_skill.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentSkill extends AgentSkill {
  @override
  final String description;
  @override
  final BuiltList<String>? examples;
  @override
  final String id;
  @override
  final BuiltList<String>? inputModes;
  @override
  final String name;
  @override
  final BuiltList<String>? outputModes;
  @override
  final BuiltList<SecurityRequirement>? securityRequirements;
  @override
  final BuiltList<String> tags;

  factory _$AgentSkill([void Function(AgentSkillBuilder)? updates]) =>
      (AgentSkillBuilder()..update(updates))._build();

  _$AgentSkill._(
      {required this.description,
      this.examples,
      required this.id,
      this.inputModes,
      required this.name,
      this.outputModes,
      this.securityRequirements,
      required this.tags})
      : super._();
  @override
  AgentSkill rebuild(void Function(AgentSkillBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentSkillBuilder toBuilder() => AgentSkillBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentSkill &&
        description == other.description &&
        examples == other.examples &&
        id == other.id &&
        inputModes == other.inputModes &&
        name == other.name &&
        outputModes == other.outputModes &&
        securityRequirements == other.securityRequirements &&
        tags == other.tags;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, examples.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, inputModes.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, outputModes.hashCode);
    _$hash = $jc(_$hash, securityRequirements.hashCode);
    _$hash = $jc(_$hash, tags.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentSkill')
          ..add('description', description)
          ..add('examples', examples)
          ..add('id', id)
          ..add('inputModes', inputModes)
          ..add('name', name)
          ..add('outputModes', outputModes)
          ..add('securityRequirements', securityRequirements)
          ..add('tags', tags))
        .toString();
  }
}

class AgentSkillBuilder implements Builder<AgentSkill, AgentSkillBuilder> {
  _$AgentSkill? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  ListBuilder<String>? _examples;
  ListBuilder<String> get examples =>
      _$this._examples ??= ListBuilder<String>();
  set examples(ListBuilder<String>? examples) => _$this._examples = examples;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  ListBuilder<String>? _inputModes;
  ListBuilder<String> get inputModes =>
      _$this._inputModes ??= ListBuilder<String>();
  set inputModes(ListBuilder<String>? inputModes) =>
      _$this._inputModes = inputModes;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ListBuilder<String>? _outputModes;
  ListBuilder<String> get outputModes =>
      _$this._outputModes ??= ListBuilder<String>();
  set outputModes(ListBuilder<String>? outputModes) =>
      _$this._outputModes = outputModes;

  ListBuilder<SecurityRequirement>? _securityRequirements;
  ListBuilder<SecurityRequirement> get securityRequirements =>
      _$this._securityRequirements ??= ListBuilder<SecurityRequirement>();
  set securityRequirements(
          ListBuilder<SecurityRequirement>? securityRequirements) =>
      _$this._securityRequirements = securityRequirements;

  ListBuilder<String>? _tags;
  ListBuilder<String> get tags => _$this._tags ??= ListBuilder<String>();
  set tags(ListBuilder<String>? tags) => _$this._tags = tags;

  AgentSkillBuilder() {
    AgentSkill._defaults(this);
  }

  AgentSkillBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _examples = $v.examples?.toBuilder();
      _id = $v.id;
      _inputModes = $v.inputModes?.toBuilder();
      _name = $v.name;
      _outputModes = $v.outputModes?.toBuilder();
      _securityRequirements = $v.securityRequirements?.toBuilder();
      _tags = $v.tags.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentSkill other) {
    _$v = other as _$AgentSkill;
  }

  @override
  void update(void Function(AgentSkillBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentSkill build() => _build();

  _$AgentSkill _build() {
    _$AgentSkill _$result;
    try {
      _$result = _$v ??
          _$AgentSkill._(
            description: BuiltValueNullFieldError.checkNotNull(
                description, r'AgentSkill', 'description'),
            examples: _examples?.build(),
            id: BuiltValueNullFieldError.checkNotNull(id, r'AgentSkill', 'id'),
            inputModes: _inputModes?.build(),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'AgentSkill', 'name'),
            outputModes: _outputModes?.build(),
            securityRequirements: _securityRequirements?.build(),
            tags: tags.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'examples';
        _examples?.build();

        _$failedField = 'inputModes';
        _inputModes?.build();

        _$failedField = 'outputModes';
        _outputModes?.build();
        _$failedField = 'securityRequirements';
        _securityRequirements?.build();
        _$failedField = 'tags';
        tags.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AgentSkill', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
