// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_key_security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApiKeySecurityScheme extends ApiKeySecurityScheme {
  @override
  final String? description;
  @override
  final String location;
  @override
  final String name;

  factory _$ApiKeySecurityScheme(
          [void Function(ApiKeySecuritySchemeBuilder)? updates]) =>
      (ApiKeySecuritySchemeBuilder()..update(updates))._build();

  _$ApiKeySecurityScheme._(
      {this.description, required this.location, required this.name})
      : super._();
  @override
  ApiKeySecurityScheme rebuild(
          void Function(ApiKeySecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApiKeySecuritySchemeBuilder toBuilder() =>
      ApiKeySecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApiKeySecurityScheme &&
        description == other.description &&
        location == other.location &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, location.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApiKeySecurityScheme')
          ..add('description', description)
          ..add('location', location)
          ..add('name', name))
        .toString();
  }
}

class ApiKeySecuritySchemeBuilder
    implements Builder<ApiKeySecurityScheme, ApiKeySecuritySchemeBuilder> {
  _$ApiKeySecurityScheme? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _location;
  String? get location => _$this._location;
  set location(String? location) => _$this._location = location;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ApiKeySecuritySchemeBuilder() {
    ApiKeySecurityScheme._defaults(this);
  }

  ApiKeySecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _location = $v.location;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApiKeySecurityScheme other) {
    _$v = other as _$ApiKeySecurityScheme;
  }

  @override
  void update(void Function(ApiKeySecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApiKeySecurityScheme build() => _build();

  _$ApiKeySecurityScheme _build() {
    final _$result = _$v ??
        _$ApiKeySecurityScheme._(
          description: description,
          location: BuiltValueNullFieldError.checkNotNull(
              location, r'ApiKeySecurityScheme', 'location'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'ApiKeySecurityScheme', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
