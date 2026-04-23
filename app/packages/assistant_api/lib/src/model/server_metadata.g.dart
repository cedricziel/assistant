// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'server_metadata.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ServerMetadata extends ServerMetadata {
  @override
  final String authorizationEndpoint;
  @override
  final BuiltList<String> codeChallengeMethodsSupported;
  @override
  final String deviceAuthorizationEndpoint;
  @override
  final BuiltList<String> grantTypesSupported;
  @override
  final String issuer;
  @override
  final String registrationEndpoint;
  @override
  final BuiltList<String> responseTypesSupported;
  @override
  final String revocationEndpoint;
  @override
  final String tokenEndpoint;
  @override
  final BuiltList<String> tokenEndpointAuthMethodsSupported;

  factory _$ServerMetadata([void Function(ServerMetadataBuilder)? updates]) =>
      (ServerMetadataBuilder()..update(updates))._build();

  _$ServerMetadata._(
      {required this.authorizationEndpoint,
      required this.codeChallengeMethodsSupported,
      required this.deviceAuthorizationEndpoint,
      required this.grantTypesSupported,
      required this.issuer,
      required this.registrationEndpoint,
      required this.responseTypesSupported,
      required this.revocationEndpoint,
      required this.tokenEndpoint,
      required this.tokenEndpointAuthMethodsSupported})
      : super._();
  @override
  ServerMetadata rebuild(void Function(ServerMetadataBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ServerMetadataBuilder toBuilder() => ServerMetadataBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ServerMetadata &&
        authorizationEndpoint == other.authorizationEndpoint &&
        codeChallengeMethodsSupported == other.codeChallengeMethodsSupported &&
        deviceAuthorizationEndpoint == other.deviceAuthorizationEndpoint &&
        grantTypesSupported == other.grantTypesSupported &&
        issuer == other.issuer &&
        registrationEndpoint == other.registrationEndpoint &&
        responseTypesSupported == other.responseTypesSupported &&
        revocationEndpoint == other.revocationEndpoint &&
        tokenEndpoint == other.tokenEndpoint &&
        tokenEndpointAuthMethodsSupported ==
            other.tokenEndpointAuthMethodsSupported;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authorizationEndpoint.hashCode);
    _$hash = $jc(_$hash, codeChallengeMethodsSupported.hashCode);
    _$hash = $jc(_$hash, deviceAuthorizationEndpoint.hashCode);
    _$hash = $jc(_$hash, grantTypesSupported.hashCode);
    _$hash = $jc(_$hash, issuer.hashCode);
    _$hash = $jc(_$hash, registrationEndpoint.hashCode);
    _$hash = $jc(_$hash, responseTypesSupported.hashCode);
    _$hash = $jc(_$hash, revocationEndpoint.hashCode);
    _$hash = $jc(_$hash, tokenEndpoint.hashCode);
    _$hash = $jc(_$hash, tokenEndpointAuthMethodsSupported.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ServerMetadata')
          ..add('authorizationEndpoint', authorizationEndpoint)
          ..add('codeChallengeMethodsSupported', codeChallengeMethodsSupported)
          ..add('deviceAuthorizationEndpoint', deviceAuthorizationEndpoint)
          ..add('grantTypesSupported', grantTypesSupported)
          ..add('issuer', issuer)
          ..add('registrationEndpoint', registrationEndpoint)
          ..add('responseTypesSupported', responseTypesSupported)
          ..add('revocationEndpoint', revocationEndpoint)
          ..add('tokenEndpoint', tokenEndpoint)
          ..add('tokenEndpointAuthMethodsSupported',
              tokenEndpointAuthMethodsSupported))
        .toString();
  }
}

class ServerMetadataBuilder
    implements Builder<ServerMetadata, ServerMetadataBuilder> {
  _$ServerMetadata? _$v;

  String? _authorizationEndpoint;
  String? get authorizationEndpoint => _$this._authorizationEndpoint;
  set authorizationEndpoint(String? authorizationEndpoint) =>
      _$this._authorizationEndpoint = authorizationEndpoint;

  ListBuilder<String>? _codeChallengeMethodsSupported;
  ListBuilder<String> get codeChallengeMethodsSupported =>
      _$this._codeChallengeMethodsSupported ??= ListBuilder<String>();
  set codeChallengeMethodsSupported(
          ListBuilder<String>? codeChallengeMethodsSupported) =>
      _$this._codeChallengeMethodsSupported = codeChallengeMethodsSupported;

  String? _deviceAuthorizationEndpoint;
  String? get deviceAuthorizationEndpoint =>
      _$this._deviceAuthorizationEndpoint;
  set deviceAuthorizationEndpoint(String? deviceAuthorizationEndpoint) =>
      _$this._deviceAuthorizationEndpoint = deviceAuthorizationEndpoint;

  ListBuilder<String>? _grantTypesSupported;
  ListBuilder<String> get grantTypesSupported =>
      _$this._grantTypesSupported ??= ListBuilder<String>();
  set grantTypesSupported(ListBuilder<String>? grantTypesSupported) =>
      _$this._grantTypesSupported = grantTypesSupported;

  String? _issuer;
  String? get issuer => _$this._issuer;
  set issuer(String? issuer) => _$this._issuer = issuer;

  String? _registrationEndpoint;
  String? get registrationEndpoint => _$this._registrationEndpoint;
  set registrationEndpoint(String? registrationEndpoint) =>
      _$this._registrationEndpoint = registrationEndpoint;

  ListBuilder<String>? _responseTypesSupported;
  ListBuilder<String> get responseTypesSupported =>
      _$this._responseTypesSupported ??= ListBuilder<String>();
  set responseTypesSupported(ListBuilder<String>? responseTypesSupported) =>
      _$this._responseTypesSupported = responseTypesSupported;

  String? _revocationEndpoint;
  String? get revocationEndpoint => _$this._revocationEndpoint;
  set revocationEndpoint(String? revocationEndpoint) =>
      _$this._revocationEndpoint = revocationEndpoint;

  String? _tokenEndpoint;
  String? get tokenEndpoint => _$this._tokenEndpoint;
  set tokenEndpoint(String? tokenEndpoint) =>
      _$this._tokenEndpoint = tokenEndpoint;

  ListBuilder<String>? _tokenEndpointAuthMethodsSupported;
  ListBuilder<String> get tokenEndpointAuthMethodsSupported =>
      _$this._tokenEndpointAuthMethodsSupported ??= ListBuilder<String>();
  set tokenEndpointAuthMethodsSupported(
          ListBuilder<String>? tokenEndpointAuthMethodsSupported) =>
      _$this._tokenEndpointAuthMethodsSupported =
          tokenEndpointAuthMethodsSupported;

  ServerMetadataBuilder() {
    ServerMetadata._defaults(this);
  }

  ServerMetadataBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authorizationEndpoint = $v.authorizationEndpoint;
      _codeChallengeMethodsSupported =
          $v.codeChallengeMethodsSupported.toBuilder();
      _deviceAuthorizationEndpoint = $v.deviceAuthorizationEndpoint;
      _grantTypesSupported = $v.grantTypesSupported.toBuilder();
      _issuer = $v.issuer;
      _registrationEndpoint = $v.registrationEndpoint;
      _responseTypesSupported = $v.responseTypesSupported.toBuilder();
      _revocationEndpoint = $v.revocationEndpoint;
      _tokenEndpoint = $v.tokenEndpoint;
      _tokenEndpointAuthMethodsSupported =
          $v.tokenEndpointAuthMethodsSupported.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ServerMetadata other) {
    _$v = other as _$ServerMetadata;
  }

  @override
  void update(void Function(ServerMetadataBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ServerMetadata build() => _build();

  _$ServerMetadata _build() {
    _$ServerMetadata _$result;
    try {
      _$result = _$v ??
          _$ServerMetadata._(
            authorizationEndpoint: BuiltValueNullFieldError.checkNotNull(
                authorizationEndpoint,
                r'ServerMetadata',
                'authorizationEndpoint'),
            codeChallengeMethodsSupported:
                codeChallengeMethodsSupported.build(),
            deviceAuthorizationEndpoint: BuiltValueNullFieldError.checkNotNull(
                deviceAuthorizationEndpoint,
                r'ServerMetadata',
                'deviceAuthorizationEndpoint'),
            grantTypesSupported: grantTypesSupported.build(),
            issuer: BuiltValueNullFieldError.checkNotNull(
                issuer, r'ServerMetadata', 'issuer'),
            registrationEndpoint: BuiltValueNullFieldError.checkNotNull(
                registrationEndpoint,
                r'ServerMetadata',
                'registrationEndpoint'),
            responseTypesSupported: responseTypesSupported.build(),
            revocationEndpoint: BuiltValueNullFieldError.checkNotNull(
                revocationEndpoint, r'ServerMetadata', 'revocationEndpoint'),
            tokenEndpoint: BuiltValueNullFieldError.checkNotNull(
                tokenEndpoint, r'ServerMetadata', 'tokenEndpoint'),
            tokenEndpointAuthMethodsSupported:
                tokenEndpointAuthMethodsSupported.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'codeChallengeMethodsSupported';
        codeChallengeMethodsSupported.build();

        _$failedField = 'grantTypesSupported';
        grantTypesSupported.build();

        _$failedField = 'responseTypesSupported';
        responseTypesSupported.build();

        _$failedField = 'tokenEndpointAuthMethodsSupported';
        tokenEndpointAuthMethodsSupported.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ServerMetadata', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
