// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'client_registration_schema.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ClientRegistrationSchema extends ClientRegistrationSchema {
  @override
  final String clientName;
  @override
  final BuiltList<String> grantTypes;
  @override
  final BuiltList<String> redirectUris;
  @override
  final BuiltList<String>? responseTypes;
  @override
  final String? tokenEndpointAuthMethod;

  factory _$ClientRegistrationSchema(
          [void Function(ClientRegistrationSchemaBuilder)? updates]) =>
      (ClientRegistrationSchemaBuilder()..update(updates))._build();

  _$ClientRegistrationSchema._(
      {required this.clientName,
      required this.grantTypes,
      required this.redirectUris,
      this.responseTypes,
      this.tokenEndpointAuthMethod})
      : super._();
  @override
  ClientRegistrationSchema rebuild(
          void Function(ClientRegistrationSchemaBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ClientRegistrationSchemaBuilder toBuilder() =>
      ClientRegistrationSchemaBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ClientRegistrationSchema &&
        clientName == other.clientName &&
        grantTypes == other.grantTypes &&
        redirectUris == other.redirectUris &&
        responseTypes == other.responseTypes &&
        tokenEndpointAuthMethod == other.tokenEndpointAuthMethod;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, clientName.hashCode);
    _$hash = $jc(_$hash, grantTypes.hashCode);
    _$hash = $jc(_$hash, redirectUris.hashCode);
    _$hash = $jc(_$hash, responseTypes.hashCode);
    _$hash = $jc(_$hash, tokenEndpointAuthMethod.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ClientRegistrationSchema')
          ..add('clientName', clientName)
          ..add('grantTypes', grantTypes)
          ..add('redirectUris', redirectUris)
          ..add('responseTypes', responseTypes)
          ..add('tokenEndpointAuthMethod', tokenEndpointAuthMethod))
        .toString();
  }
}

class ClientRegistrationSchemaBuilder
    implements
        Builder<ClientRegistrationSchema, ClientRegistrationSchemaBuilder> {
  _$ClientRegistrationSchema? _$v;

  String? _clientName;
  String? get clientName => _$this._clientName;
  set clientName(String? clientName) => _$this._clientName = clientName;

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

  ListBuilder<String>? _responseTypes;
  ListBuilder<String> get responseTypes =>
      _$this._responseTypes ??= ListBuilder<String>();
  set responseTypes(ListBuilder<String>? responseTypes) =>
      _$this._responseTypes = responseTypes;

  String? _tokenEndpointAuthMethod;
  String? get tokenEndpointAuthMethod => _$this._tokenEndpointAuthMethod;
  set tokenEndpointAuthMethod(String? tokenEndpointAuthMethod) =>
      _$this._tokenEndpointAuthMethod = tokenEndpointAuthMethod;

  ClientRegistrationSchemaBuilder() {
    ClientRegistrationSchema._defaults(this);
  }

  ClientRegistrationSchemaBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _clientName = $v.clientName;
      _grantTypes = $v.grantTypes.toBuilder();
      _redirectUris = $v.redirectUris.toBuilder();
      _responseTypes = $v.responseTypes?.toBuilder();
      _tokenEndpointAuthMethod = $v.tokenEndpointAuthMethod;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ClientRegistrationSchema other) {
    _$v = other as _$ClientRegistrationSchema;
  }

  @override
  void update(void Function(ClientRegistrationSchemaBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ClientRegistrationSchema build() => _build();

  _$ClientRegistrationSchema _build() {
    _$ClientRegistrationSchema _$result;
    try {
      _$result = _$v ??
          _$ClientRegistrationSchema._(
            clientName: BuiltValueNullFieldError.checkNotNull(
                clientName, r'ClientRegistrationSchema', 'clientName'),
            grantTypes: grantTypes.build(),
            redirectUris: redirectUris.build(),
            responseTypes: _responseTypes?.build(),
            tokenEndpointAuthMethod: tokenEndpointAuthMethod,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'grantTypes';
        grantTypes.build();
        _$failedField = 'redirectUris';
        redirectUris.build();
        _$failedField = 'responseTypes';
        _responseTypes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ClientRegistrationSchema', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
