// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_from_template_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaFromTemplateResponse extends PersonaFromTemplateResponse {
  @override
  final String name;
  @override
  final String personaId;

  factory _$PersonaFromTemplateResponse(
          [void Function(PersonaFromTemplateResponseBuilder)? updates]) =>
      (PersonaFromTemplateResponseBuilder()..update(updates))._build();

  _$PersonaFromTemplateResponse._({required this.name, required this.personaId})
      : super._();
  @override
  PersonaFromTemplateResponse rebuild(
          void Function(PersonaFromTemplateResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaFromTemplateResponseBuilder toBuilder() =>
      PersonaFromTemplateResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaFromTemplateResponse &&
        name == other.name &&
        personaId == other.personaId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaFromTemplateResponse')
          ..add('name', name)
          ..add('personaId', personaId))
        .toString();
  }
}

class PersonaFromTemplateResponseBuilder
    implements
        Builder<PersonaFromTemplateResponse,
            PersonaFromTemplateResponseBuilder> {
  _$PersonaFromTemplateResponse? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  PersonaFromTemplateResponseBuilder() {
    PersonaFromTemplateResponse._defaults(this);
  }

  PersonaFromTemplateResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _personaId = $v.personaId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaFromTemplateResponse other) {
    _$v = other as _$PersonaFromTemplateResponse;
  }

  @override
  void update(void Function(PersonaFromTemplateResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaFromTemplateResponse build() => _build();

  _$PersonaFromTemplateResponse _build() {
    final _$result = _$v ??
        _$PersonaFromTemplateResponse._(
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'PersonaFromTemplateResponse', 'name'),
          personaId: BuiltValueNullFieldError.checkNotNull(
              personaId, r'PersonaFromTemplateResponse', 'personaId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
