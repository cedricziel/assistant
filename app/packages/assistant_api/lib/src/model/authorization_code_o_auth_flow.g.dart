// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'authorization_code_o_auth_flow.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AuthorizationCodeOAuthFlow extends AuthorizationCodeOAuthFlow {
  @override
  final String authorizationUrl;
  @override
  final bool? pkceRequired;
  @override
  final String? refreshUrl;
  @override
  final BuiltMap<String, String> scopes;
  @override
  final String tokenUrl;

  factory _$AuthorizationCodeOAuthFlow(
          [void Function(AuthorizationCodeOAuthFlowBuilder)? updates]) =>
      (AuthorizationCodeOAuthFlowBuilder()..update(updates))._build();

  _$AuthorizationCodeOAuthFlow._(
      {required this.authorizationUrl,
      this.pkceRequired,
      this.refreshUrl,
      required this.scopes,
      required this.tokenUrl})
      : super._();
  @override
  AuthorizationCodeOAuthFlow rebuild(
          void Function(AuthorizationCodeOAuthFlowBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AuthorizationCodeOAuthFlowBuilder toBuilder() =>
      AuthorizationCodeOAuthFlowBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AuthorizationCodeOAuthFlow &&
        authorizationUrl == other.authorizationUrl &&
        pkceRequired == other.pkceRequired &&
        refreshUrl == other.refreshUrl &&
        scopes == other.scopes &&
        tokenUrl == other.tokenUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authorizationUrl.hashCode);
    _$hash = $jc(_$hash, pkceRequired.hashCode);
    _$hash = $jc(_$hash, refreshUrl.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jc(_$hash, tokenUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AuthorizationCodeOAuthFlow')
          ..add('authorizationUrl', authorizationUrl)
          ..add('pkceRequired', pkceRequired)
          ..add('refreshUrl', refreshUrl)
          ..add('scopes', scopes)
          ..add('tokenUrl', tokenUrl))
        .toString();
  }
}

class AuthorizationCodeOAuthFlowBuilder
    implements
        Builder<AuthorizationCodeOAuthFlow, AuthorizationCodeOAuthFlowBuilder> {
  _$AuthorizationCodeOAuthFlow? _$v;

  String? _authorizationUrl;
  String? get authorizationUrl => _$this._authorizationUrl;
  set authorizationUrl(String? authorizationUrl) =>
      _$this._authorizationUrl = authorizationUrl;

  bool? _pkceRequired;
  bool? get pkceRequired => _$this._pkceRequired;
  set pkceRequired(bool? pkceRequired) => _$this._pkceRequired = pkceRequired;

  String? _refreshUrl;
  String? get refreshUrl => _$this._refreshUrl;
  set refreshUrl(String? refreshUrl) => _$this._refreshUrl = refreshUrl;

  MapBuilder<String, String>? _scopes;
  MapBuilder<String, String> get scopes =>
      _$this._scopes ??= MapBuilder<String, String>();
  set scopes(MapBuilder<String, String>? scopes) => _$this._scopes = scopes;

  String? _tokenUrl;
  String? get tokenUrl => _$this._tokenUrl;
  set tokenUrl(String? tokenUrl) => _$this._tokenUrl = tokenUrl;

  AuthorizationCodeOAuthFlowBuilder() {
    AuthorizationCodeOAuthFlow._defaults(this);
  }

  AuthorizationCodeOAuthFlowBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authorizationUrl = $v.authorizationUrl;
      _pkceRequired = $v.pkceRequired;
      _refreshUrl = $v.refreshUrl;
      _scopes = $v.scopes.toBuilder();
      _tokenUrl = $v.tokenUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AuthorizationCodeOAuthFlow other) {
    _$v = other as _$AuthorizationCodeOAuthFlow;
  }

  @override
  void update(void Function(AuthorizationCodeOAuthFlowBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AuthorizationCodeOAuthFlow build() => _build();

  _$AuthorizationCodeOAuthFlow _build() {
    _$AuthorizationCodeOAuthFlow _$result;
    try {
      _$result = _$v ??
          _$AuthorizationCodeOAuthFlow._(
            authorizationUrl: BuiltValueNullFieldError.checkNotNull(
                authorizationUrl,
                r'AuthorizationCodeOAuthFlow',
                'authorizationUrl'),
            pkceRequired: pkceRequired,
            refreshUrl: refreshUrl,
            scopes: scopes.build(),
            tokenUrl: BuiltValueNullFieldError.checkNotNull(
                tokenUrl, r'AuthorizationCodeOAuthFlow', 'tokenUrl'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        scopes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AuthorizationCodeOAuthFlow', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
