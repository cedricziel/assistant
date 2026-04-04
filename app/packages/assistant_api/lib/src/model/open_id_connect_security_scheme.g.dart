// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'open_id_connect_security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OpenIdConnectSecurityScheme extends OpenIdConnectSecurityScheme {
  @override
  final String? description;
  @override
  final String openIdConnectUrl;

  factory _$OpenIdConnectSecurityScheme(
          [void Function(OpenIdConnectSecuritySchemeBuilder)? updates]) =>
      (OpenIdConnectSecuritySchemeBuilder()..update(updates))._build();

  _$OpenIdConnectSecurityScheme._(
      {this.description, required this.openIdConnectUrl})
      : super._();
  @override
  OpenIdConnectSecurityScheme rebuild(
          void Function(OpenIdConnectSecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OpenIdConnectSecuritySchemeBuilder toBuilder() =>
      OpenIdConnectSecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OpenIdConnectSecurityScheme &&
        description == other.description &&
        openIdConnectUrl == other.openIdConnectUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, openIdConnectUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OpenIdConnectSecurityScheme')
          ..add('description', description)
          ..add('openIdConnectUrl', openIdConnectUrl))
        .toString();
  }
}

class OpenIdConnectSecuritySchemeBuilder
    implements
        Builder<OpenIdConnectSecurityScheme,
            OpenIdConnectSecuritySchemeBuilder> {
  _$OpenIdConnectSecurityScheme? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _openIdConnectUrl;
  String? get openIdConnectUrl => _$this._openIdConnectUrl;
  set openIdConnectUrl(String? openIdConnectUrl) =>
      _$this._openIdConnectUrl = openIdConnectUrl;

  OpenIdConnectSecuritySchemeBuilder() {
    OpenIdConnectSecurityScheme._defaults(this);
  }

  OpenIdConnectSecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _openIdConnectUrl = $v.openIdConnectUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OpenIdConnectSecurityScheme other) {
    _$v = other as _$OpenIdConnectSecurityScheme;
  }

  @override
  void update(void Function(OpenIdConnectSecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OpenIdConnectSecurityScheme build() => _build();

  _$OpenIdConnectSecurityScheme _build() {
    final _$result = _$v ??
        _$OpenIdConnectSecurityScheme._(
          description: description,
          openIdConnectUrl: BuiltValueNullFieldError.checkNotNull(
              openIdConnectUrl,
              r'OpenIdConnectSecurityScheme',
              'openIdConnectUrl'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
