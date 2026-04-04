// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'o_auth_flows.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OAuthFlows extends OAuthFlows {
  @override
  final AuthorizationCodeOAuthFlow? authorizationCode;
  @override
  final ClientCredentialsOAuthFlow? clientCredentials;
  @override
  final DeviceCodeOAuthFlow? deviceCode;
  @override
  final ImplicitOAuthFlow? implicit;
  @override
  final PasswordOAuthFlow? password;

  factory _$OAuthFlows([void Function(OAuthFlowsBuilder)? updates]) =>
      (OAuthFlowsBuilder()..update(updates))._build();

  _$OAuthFlows._(
      {this.authorizationCode,
      this.clientCredentials,
      this.deviceCode,
      this.implicit,
      this.password})
      : super._();
  @override
  OAuthFlows rebuild(void Function(OAuthFlowsBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OAuthFlowsBuilder toBuilder() => OAuthFlowsBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OAuthFlows &&
        authorizationCode == other.authorizationCode &&
        clientCredentials == other.clientCredentials &&
        deviceCode == other.deviceCode &&
        implicit == other.implicit &&
        password == other.password;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authorizationCode.hashCode);
    _$hash = $jc(_$hash, clientCredentials.hashCode);
    _$hash = $jc(_$hash, deviceCode.hashCode);
    _$hash = $jc(_$hash, implicit.hashCode);
    _$hash = $jc(_$hash, password.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OAuthFlows')
          ..add('authorizationCode', authorizationCode)
          ..add('clientCredentials', clientCredentials)
          ..add('deviceCode', deviceCode)
          ..add('implicit', implicit)
          ..add('password', password))
        .toString();
  }
}

class OAuthFlowsBuilder implements Builder<OAuthFlows, OAuthFlowsBuilder> {
  _$OAuthFlows? _$v;

  AuthorizationCodeOAuthFlowBuilder? _authorizationCode;
  AuthorizationCodeOAuthFlowBuilder get authorizationCode =>
      _$this._authorizationCode ??= AuthorizationCodeOAuthFlowBuilder();
  set authorizationCode(AuthorizationCodeOAuthFlowBuilder? authorizationCode) =>
      _$this._authorizationCode = authorizationCode;

  ClientCredentialsOAuthFlowBuilder? _clientCredentials;
  ClientCredentialsOAuthFlowBuilder get clientCredentials =>
      _$this._clientCredentials ??= ClientCredentialsOAuthFlowBuilder();
  set clientCredentials(ClientCredentialsOAuthFlowBuilder? clientCredentials) =>
      _$this._clientCredentials = clientCredentials;

  DeviceCodeOAuthFlowBuilder? _deviceCode;
  DeviceCodeOAuthFlowBuilder get deviceCode =>
      _$this._deviceCode ??= DeviceCodeOAuthFlowBuilder();
  set deviceCode(DeviceCodeOAuthFlowBuilder? deviceCode) =>
      _$this._deviceCode = deviceCode;

  ImplicitOAuthFlowBuilder? _implicit;
  ImplicitOAuthFlowBuilder get implicit =>
      _$this._implicit ??= ImplicitOAuthFlowBuilder();
  set implicit(ImplicitOAuthFlowBuilder? implicit) =>
      _$this._implicit = implicit;

  PasswordOAuthFlowBuilder? _password;
  PasswordOAuthFlowBuilder get password =>
      _$this._password ??= PasswordOAuthFlowBuilder();
  set password(PasswordOAuthFlowBuilder? password) =>
      _$this._password = password;

  OAuthFlowsBuilder() {
    OAuthFlows._defaults(this);
  }

  OAuthFlowsBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authorizationCode = $v.authorizationCode?.toBuilder();
      _clientCredentials = $v.clientCredentials?.toBuilder();
      _deviceCode = $v.deviceCode?.toBuilder();
      _implicit = $v.implicit?.toBuilder();
      _password = $v.password?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OAuthFlows other) {
    _$v = other as _$OAuthFlows;
  }

  @override
  void update(void Function(OAuthFlowsBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OAuthFlows build() => _build();

  _$OAuthFlows _build() {
    _$OAuthFlows _$result;
    try {
      _$result = _$v ??
          _$OAuthFlows._(
            authorizationCode: _authorizationCode?.build(),
            clientCredentials: _clientCredentials?.build(),
            deviceCode: _deviceCode?.build(),
            implicit: _implicit?.build(),
            password: _password?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'authorizationCode';
        _authorizationCode?.build();
        _$failedField = 'clientCredentials';
        _clientCredentials?.build();
        _$failedField = 'deviceCode';
        _deviceCode?.build();
        _$failedField = 'implicit';
        _implicit?.build();
        _$failedField = 'password';
        _password?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'OAuthFlows', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
