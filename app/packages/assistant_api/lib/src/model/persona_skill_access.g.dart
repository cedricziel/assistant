// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_skill_access.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaSkillAccess extends PersonaSkillAccess {
  @override
  final String mode;
  @override
  final BuiltList<String> skills;

  factory _$PersonaSkillAccess(
          [void Function(PersonaSkillAccessBuilder)? updates]) =>
      (PersonaSkillAccessBuilder()..update(updates))._build();

  _$PersonaSkillAccess._({required this.mode, required this.skills})
      : super._();
  @override
  PersonaSkillAccess rebuild(
          void Function(PersonaSkillAccessBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaSkillAccessBuilder toBuilder() =>
      PersonaSkillAccessBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaSkillAccess &&
        mode == other.mode &&
        skills == other.skills;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, mode.hashCode);
    _$hash = $jc(_$hash, skills.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaSkillAccess')
          ..add('mode', mode)
          ..add('skills', skills))
        .toString();
  }
}

class PersonaSkillAccessBuilder
    implements Builder<PersonaSkillAccess, PersonaSkillAccessBuilder> {
  _$PersonaSkillAccess? _$v;

  String? _mode;
  String? get mode => _$this._mode;
  set mode(String? mode) => _$this._mode = mode;

  ListBuilder<String>? _skills;
  ListBuilder<String> get skills => _$this._skills ??= ListBuilder<String>();
  set skills(ListBuilder<String>? skills) => _$this._skills = skills;

  PersonaSkillAccessBuilder() {
    PersonaSkillAccess._defaults(this);
  }

  PersonaSkillAccessBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _mode = $v.mode;
      _skills = $v.skills.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaSkillAccess other) {
    _$v = other as _$PersonaSkillAccess;
  }

  @override
  void update(void Function(PersonaSkillAccessBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaSkillAccess build() => _build();

  _$PersonaSkillAccess _build() {
    _$PersonaSkillAccess _$result;
    try {
      _$result = _$v ??
          _$PersonaSkillAccess._(
            mode: BuiltValueNullFieldError.checkNotNull(
                mode, r'PersonaSkillAccess', 'mode'),
            skills: skills.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'skills';
        skills.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'PersonaSkillAccess', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
