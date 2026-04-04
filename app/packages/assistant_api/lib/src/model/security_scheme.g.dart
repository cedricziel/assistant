// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'security_scheme.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SecurityScheme extends SecurityScheme {
  @override
  final ApiKeySecurityScheme? apiKeySecurityScheme;
  @override
  final HttpAuthSecurityScheme? httpAuthSecurityScheme;
  @override
  final MutualTlsSecurityScheme? mtlsSecurityScheme;
  @override
  final OAuth2SecurityScheme? oauth2SecurityScheme;
  @override
  final OpenIdConnectSecurityScheme? openIdConnectSecurityScheme;

  factory _$SecurityScheme([void Function(SecuritySchemeBuilder)? updates]) =>
      (SecuritySchemeBuilder()..update(updates))._build();

  _$SecurityScheme._(
      {this.apiKeySecurityScheme,
      this.httpAuthSecurityScheme,
      this.mtlsSecurityScheme,
      this.oauth2SecurityScheme,
      this.openIdConnectSecurityScheme})
      : super._();
  @override
  SecurityScheme rebuild(void Function(SecuritySchemeBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SecuritySchemeBuilder toBuilder() => SecuritySchemeBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SecurityScheme &&
        apiKeySecurityScheme == other.apiKeySecurityScheme &&
        httpAuthSecurityScheme == other.httpAuthSecurityScheme &&
        mtlsSecurityScheme == other.mtlsSecurityScheme &&
        oauth2SecurityScheme == other.oauth2SecurityScheme &&
        openIdConnectSecurityScheme == other.openIdConnectSecurityScheme;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, apiKeySecurityScheme.hashCode);
    _$hash = $jc(_$hash, httpAuthSecurityScheme.hashCode);
    _$hash = $jc(_$hash, mtlsSecurityScheme.hashCode);
    _$hash = $jc(_$hash, oauth2SecurityScheme.hashCode);
    _$hash = $jc(_$hash, openIdConnectSecurityScheme.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SecurityScheme')
          ..add('apiKeySecurityScheme', apiKeySecurityScheme)
          ..add('httpAuthSecurityScheme', httpAuthSecurityScheme)
          ..add('mtlsSecurityScheme', mtlsSecurityScheme)
          ..add('oauth2SecurityScheme', oauth2SecurityScheme)
          ..add('openIdConnectSecurityScheme', openIdConnectSecurityScheme))
        .toString();
  }
}

class SecuritySchemeBuilder
    implements Builder<SecurityScheme, SecuritySchemeBuilder> {
  _$SecurityScheme? _$v;

  ApiKeySecuritySchemeBuilder? _apiKeySecurityScheme;
  ApiKeySecuritySchemeBuilder get apiKeySecurityScheme =>
      _$this._apiKeySecurityScheme ??= ApiKeySecuritySchemeBuilder();
  set apiKeySecurityScheme(ApiKeySecuritySchemeBuilder? apiKeySecurityScheme) =>
      _$this._apiKeySecurityScheme = apiKeySecurityScheme;

  HttpAuthSecuritySchemeBuilder? _httpAuthSecurityScheme;
  HttpAuthSecuritySchemeBuilder get httpAuthSecurityScheme =>
      _$this._httpAuthSecurityScheme ??= HttpAuthSecuritySchemeBuilder();
  set httpAuthSecurityScheme(
          HttpAuthSecuritySchemeBuilder? httpAuthSecurityScheme) =>
      _$this._httpAuthSecurityScheme = httpAuthSecurityScheme;

  MutualTlsSecuritySchemeBuilder? _mtlsSecurityScheme;
  MutualTlsSecuritySchemeBuilder get mtlsSecurityScheme =>
      _$this._mtlsSecurityScheme ??= MutualTlsSecuritySchemeBuilder();
  set mtlsSecurityScheme(MutualTlsSecuritySchemeBuilder? mtlsSecurityScheme) =>
      _$this._mtlsSecurityScheme = mtlsSecurityScheme;

  OAuth2SecuritySchemeBuilder? _oauth2SecurityScheme;
  OAuth2SecuritySchemeBuilder get oauth2SecurityScheme =>
      _$this._oauth2SecurityScheme ??= OAuth2SecuritySchemeBuilder();
  set oauth2SecurityScheme(OAuth2SecuritySchemeBuilder? oauth2SecurityScheme) =>
      _$this._oauth2SecurityScheme = oauth2SecurityScheme;

  OpenIdConnectSecuritySchemeBuilder? _openIdConnectSecurityScheme;
  OpenIdConnectSecuritySchemeBuilder get openIdConnectSecurityScheme =>
      _$this._openIdConnectSecurityScheme ??=
          OpenIdConnectSecuritySchemeBuilder();
  set openIdConnectSecurityScheme(
          OpenIdConnectSecuritySchemeBuilder? openIdConnectSecurityScheme) =>
      _$this._openIdConnectSecurityScheme = openIdConnectSecurityScheme;

  SecuritySchemeBuilder() {
    SecurityScheme._defaults(this);
  }

  SecuritySchemeBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _apiKeySecurityScheme = $v.apiKeySecurityScheme?.toBuilder();
      _httpAuthSecurityScheme = $v.httpAuthSecurityScheme?.toBuilder();
      _mtlsSecurityScheme = $v.mtlsSecurityScheme?.toBuilder();
      _oauth2SecurityScheme = $v.oauth2SecurityScheme?.toBuilder();
      _openIdConnectSecurityScheme =
          $v.openIdConnectSecurityScheme?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SecurityScheme other) {
    _$v = other as _$SecurityScheme;
  }

  @override
  void update(void Function(SecuritySchemeBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SecurityScheme build() => _build();

  _$SecurityScheme _build() {
    _$SecurityScheme _$result;
    try {
      _$result = _$v ??
          _$SecurityScheme._(
            apiKeySecurityScheme: _apiKeySecurityScheme?.build(),
            httpAuthSecurityScheme: _httpAuthSecurityScheme?.build(),
            mtlsSecurityScheme: _mtlsSecurityScheme?.build(),
            oauth2SecurityScheme: _oauth2SecurityScheme?.build(),
            openIdConnectSecurityScheme: _openIdConnectSecurityScheme?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'apiKeySecurityScheme';
        _apiKeySecurityScheme?.build();
        _$failedField = 'httpAuthSecurityScheme';
        _httpAuthSecurityScheme?.build();
        _$failedField = 'mtlsSecurityScheme';
        _mtlsSecurityScheme?.build();
        _$failedField = 'oauth2SecurityScheme';
        _oauth2SecurityScheme?.build();
        _$failedField = 'openIdConnectSecurityScheme';
        _openIdConnectSecurityScheme?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SecurityScheme', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
