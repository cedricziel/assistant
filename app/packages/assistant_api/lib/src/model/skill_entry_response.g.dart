// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'skill_entry_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SkillEntryResponse extends SkillEntryResponse {
  @override
  final String description;
  @override
  final bool enabled;
  @override
  final String name;

  factory _$SkillEntryResponse(
          [void Function(SkillEntryResponseBuilder)? updates]) =>
      (SkillEntryResponseBuilder()..update(updates))._build();

  _$SkillEntryResponse._(
      {required this.description, required this.enabled, required this.name})
      : super._();
  @override
  SkillEntryResponse rebuild(
          void Function(SkillEntryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SkillEntryResponseBuilder toBuilder() =>
      SkillEntryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SkillEntryResponse &&
        description == other.description &&
        enabled == other.enabled &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SkillEntryResponse')
          ..add('description', description)
          ..add('enabled', enabled)
          ..add('name', name))
        .toString();
  }
}

class SkillEntryResponseBuilder
    implements Builder<SkillEntryResponse, SkillEntryResponseBuilder> {
  _$SkillEntryResponse? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  bool? _enabled;
  bool? get enabled => _$this._enabled;
  set enabled(bool? enabled) => _$this._enabled = enabled;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  SkillEntryResponseBuilder() {
    SkillEntryResponse._defaults(this);
  }

  SkillEntryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _enabled = $v.enabled;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SkillEntryResponse other) {
    _$v = other as _$SkillEntryResponse;
  }

  @override
  void update(void Function(SkillEntryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SkillEntryResponse build() => _build();

  _$SkillEntryResponse _build() {
    final _$result = _$v ??
        _$SkillEntryResponse._(
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'SkillEntryResponse', 'description'),
          enabled: BuiltValueNullFieldError.checkNotNull(
              enabled, r'SkillEntryResponse', 'enabled'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SkillEntryResponse', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
