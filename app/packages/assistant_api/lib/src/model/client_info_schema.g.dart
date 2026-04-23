// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'client_info_schema.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ClientInfoSchema extends ClientInfoSchema {
  @override
  final String clientId;
  @override
  final String clientName;
  @override
  final String? clientSecret;
  @override
  final BuiltList<String> grantTypes;
  @override
  final BuiltList<String> redirectUris;
  @override
  final String tokenEndpointAuthMethod;

  factory _$ClientInfoSchema(
          [void Function(ClientInfoSchemaBuilder)? updates]) =>
      (ClientInfoSchemaBuilder()..update(updates))._build();

  _$ClientInfoSchema._(
      {required this.clientId,
      required this.clientName,
      this.clientSecret,
      required this.grantTypes,
      required this.redirectUris,
      required this.tokenEndpointAuthMethod})
      : super._();
  @override
  ClientInfoSchema rebuild(void Function(ClientInfoSchemaBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ClientInfoSchemaBuilder toBuilder() =>
      ClientInfoSchemaBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ClientInfoSchema &&
        clientId == other.clientId &&
        clientName == other.clientName &&
        clientSecret == other.clientSecret &&
        grantTypes == other.grantTypes &&
        redirectUris == other.redirectUris &&
        tokenEndpointAuthMethod == other.tokenEndpointAuthMethod;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, clientId.hashCode);
    _$hash = $jc(_$hash, clientName.hashCode);
    _$hash = $jc(_$hash, clientSecret.hashCode);
    _$hash = $jc(_$hash, grantTypes.hashCode);
    _$hash = $jc(_$hash, redirectUris.hashCode);
    _$hash = $jc(_$hash, tokenEndpointAuthMethod.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ClientInfoSchema')
          ..add('clientId', clientId)
          ..add('clientName', clientName)
          ..add('clientSecret', clientSecret)
          ..add('grantTypes', grantTypes)
          ..add('redirectUris', redirectUris)
          ..add('tokenEndpointAuthMethod', tokenEndpointAuthMethod))
        .toString();
  }
}

class ClientInfoSchemaBuilder
    implements Builder<ClientInfoSchema, ClientInfoSchemaBuilder> {
  _$ClientInfoSchema? _$v;

  String? _clientId;
  String? get clientId => _$this._clientId;
  set clientId(String? clientId) => _$this._clientId = clientId;

  String? _clientName;
  String? get clientName => _$this._clientName;
  set clientName(String? clientName) => _$this._clientName = clientName;

  String? _clientSecret;
  String? get clientSecret => _$this._clientSecret;
  set clientSecret(String? clientSecret) => _$this._clientSecret = clientSecret;

  ListBuilder<String>? _grantTypes;
  ListBuilder<String> get grantTypes =>
      _$this._grantTypes ??= ListBuilder<String>();
  set grantTypes(ListBuilder<String>? grantTypes) =>
      _$this._grantTypes = grantTypes;

  ListBuilder<String>? _redirectUris;
  ListBuilder<String> get redirectUris =>
      _$this._redirectUris ??= ListBuilder<String>();
  set redirectUris(ListBuilder<String>? redirectUris) =>
      _$this._redirectUris = redirectUris;

  String? _tokenEndpointAuthMethod;
  String? get tokenEndpointAuthMethod => _$this._tokenEndpointAuthMethod;
  set tokenEndpointAuthMethod(String? tokenEndpointAuthMethod) =>
      _$this._tokenEndpointAuthMethod = tokenEndpointAuthMethod;

  ClientInfoSchemaBuilder() {
    ClientInfoSchema._defaults(this);
  }

  ClientInfoSchemaBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _clientId = $v.clientId;
      _clientName = $v.clientName;
      _clientSecret = $v.clientSecret;
      _grantTypes = $v.grantTypes.toBuilder();
      _redirectUris = $v.redirectUris.toBuilder();
      _tokenEndpointAuthMethod = $v.tokenEndpointAuthMethod;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ClientInfoSchema other) {
    _$v = other as _$ClientInfoSchema;
  }

  @override
  void update(void Function(ClientInfoSchemaBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ClientInfoSchema build() => _build();

  _$ClientInfoSchema _build() {
    _$ClientInfoSchema _$result;
    try {
      _$result = _$v ??
          _$ClientInfoSchema._(
            clientId: BuiltValueNullFieldError.checkNotNull(
                clientId, r'ClientInfoSchema', 'clientId'),
            clientName: BuiltValueNullFieldError.checkNotNull(
                clientName, r'ClientInfoSchema', 'clientName'),
            clientSecret: clientSecret,
            grantTypes: grantTypes.build(),
            redirectUris: redirectUris.build(),
            tokenEndpointAuthMethod: BuiltValueNullFieldError.checkNotNull(
                tokenEndpointAuthMethod,
                r'ClientInfoSchema',
                'tokenEndpointAuthMethod'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'grantTypes';
        grantTypes.build();
        _$failedField = 'redirectUris';
        redirectUris.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ClientInfoSchema', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
