// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_integration_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateIntegrationRequest extends CreateIntegrationRequest {
  @override
  final IntegrationAuthKind? authKind;
  @override
  final String? catalogEntryId;
  @override
  final String? connectUrl;
  @override
  final CredentialMode? credentialMode;
  @override
  final bool? enabled;
  @override
  final String? headerName;
  @override
  final String? headerValuePrefix;
  @override
  final IntegrationLayer layer;
  @override
  final String? name;
  @override
  final String? slug;
  @override
  final String? spaceId;
  @override
  final IntegrationTrust? trust;

  factory _$CreateIntegrationRequest(
          [void Function(CreateIntegrationRequestBuilder)? updates]) =>
      (CreateIntegrationRequestBuilder()..update(updates))._build();

  _$CreateIntegrationRequest._(
      {this.authKind,
      this.catalogEntryId,
      this.connectUrl,
      this.credentialMode,
      this.enabled,
      this.headerName,
      this.headerValuePrefix,
      required this.layer,
      this.name,
      this.slug,
      this.spaceId,
      this.trust})
      : super._();
  @override
  CreateIntegrationRequest rebuild(
          void Function(CreateIntegrationRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateIntegrationRequestBuilder toBuilder() =>
      CreateIntegrationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateIntegrationRequest &&
        authKind == other.authKind &&
        catalogEntryId == other.catalogEntryId &&
        connectUrl == other.connectUrl &&
        credentialMode == other.credentialMode &&
        enabled == other.enabled &&
        headerName == other.headerName &&
        headerValuePrefix == other.headerValuePrefix &&
        layer == other.layer &&
        name == other.name &&
        slug == other.slug &&
        spaceId == other.spaceId &&
        trust == other.trust;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authKind.hashCode);
    _$hash = $jc(_$hash, catalogEntryId.hashCode);
    _$hash = $jc(_$hash, connectUrl.hashCode);
    _$hash = $jc(_$hash, credentialMode.hashCode);
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jc(_$hash, headerName.hashCode);
    _$hash = $jc(_$hash, headerValuePrefix.hashCode);
    _$hash = $jc(_$hash, layer.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jc(_$hash, trust.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateIntegrationRequest')
          ..add('authKind', authKind)
          ..add('catalogEntryId', catalogEntryId)
          ..add('connectUrl', connectUrl)
          ..add('credentialMode', credentialMode)
          ..add('enabled', enabled)
          ..add('headerName', headerName)
          ..add('headerValuePrefix', headerValuePrefix)
          ..add('layer', layer)
          ..add('name', name)
          ..add('slug', slug)
          ..add('spaceId', spaceId)
          ..add('trust', trust))
        .toString();
  }
}

class CreateIntegrationRequestBuilder
    implements
        Builder<CreateIntegrationRequest, CreateIntegrationRequestBuilder> {
  _$CreateIntegrationRequest? _$v;

  IntegrationAuthKind? _authKind;
  IntegrationAuthKind? get authKind => _$this._authKind;
  set authKind(IntegrationAuthKind? authKind) => _$this._authKind = authKind;

  String? _catalogEntryId;
  String? get catalogEntryId => _$this._catalogEntryId;
  set catalogEntryId(String? catalogEntryId) =>
      _$this._catalogEntryId = catalogEntryId;

  String? _connectUrl;
  String? get connectUrl => _$this._connectUrl;
  set connectUrl(String? connectUrl) => _$this._connectUrl = connectUrl;

  CredentialMode? _credentialMode;
  CredentialMode? get credentialMode => _$this._credentialMode;
  set credentialMode(CredentialMode? credentialMode) =>
      _$this._credentialMode = credentialMode;

  bool? _enabled;
  bool? get enabled => _$this._enabled;
  set enabled(bool? enabled) => _$this._enabled = enabled;

  String? _headerName;
  String? get headerName => _$this._headerName;
  set headerName(String? headerName) => _$this._headerName = headerName;

  String? _headerValuePrefix;
  String? get headerValuePrefix => _$this._headerValuePrefix;
  set headerValuePrefix(String? headerValuePrefix) =>
      _$this._headerValuePrefix = headerValuePrefix;

  IntegrationLayer? _layer;
  IntegrationLayer? get layer => _$this._layer;
  set layer(IntegrationLayer? layer) => _$this._layer = layer;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  IntegrationTrust? _trust;
  IntegrationTrust? get trust => _$this._trust;
  set trust(IntegrationTrust? trust) => _$this._trust = trust;

  CreateIntegrationRequestBuilder() {
    CreateIntegrationRequest._defaults(this);
  }

  CreateIntegrationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authKind = $v.authKind;
      _catalogEntryId = $v.catalogEntryId;
      _connectUrl = $v.connectUrl;
      _credentialMode = $v.credentialMode;
      _enabled = $v.enabled;
      _headerName = $v.headerName;
      _headerValuePrefix = $v.headerValuePrefix;
      _layer = $v.layer;
      _name = $v.name;
      _slug = $v.slug;
      _spaceId = $v.spaceId;
      _trust = $v.trust;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateIntegrationRequest other) {
    _$v = other as _$CreateIntegrationRequest;
  }

  @override
  void update(void Function(CreateIntegrationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateIntegrationRequest build() => _build();

  _$CreateIntegrationRequest _build() {
    final _$result = _$v ??
        _$CreateIntegrationRequest._(
          authKind: authKind,
          catalogEntryId: catalogEntryId,
          connectUrl: connectUrl,
          credentialMode: credentialMode,
          enabled: enabled,
          headerName: headerName,
          headerValuePrefix: headerValuePrefix,
          layer: BuiltValueNullFieldError.checkNotNull(
              layer, r'CreateIntegrationRequest', 'layer'),
          name: name,
          slug: slug,
          spaceId: spaceId,
          trust: trust,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
