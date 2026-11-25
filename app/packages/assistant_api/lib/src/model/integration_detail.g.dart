// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$IntegrationDetail extends IntegrationDetail {
  @override
  final IntegrationAuthKind authKind;
  @override
  final String? catalogEntryId;
  @override
  final String connectUrl;
  @override
  final DateTime createdAt;
  @override
  final CredentialMode credentialMode;
  @override
  final bool enabled;
  @override
  final bool hasCredential;
  @override
  final String? headerName;
  @override
  final String? headerValuePrefix;
  @override
  final String id;
  @override
  final IntegrationLayer layer;
  @override
  final String name;
  @override
  final String? ownerUserId;
  @override
  final String slug;
  @override
  final BuiltList<SpaceAccessEntry> spaceAccess;
  @override
  final String? spaceId;
  @override
  final IntegrationStatus status;
  @override
  final IntegrationTrust trust;
  @override
  final DateTime updatedAt;

  factory _$IntegrationDetail(
          [void Function(IntegrationDetailBuilder)? updates]) =>
      (IntegrationDetailBuilder()..update(updates))._build();

  _$IntegrationDetail._(
      {required this.authKind,
      this.catalogEntryId,
      required this.connectUrl,
      required this.createdAt,
      required this.credentialMode,
      required this.enabled,
      required this.hasCredential,
      this.headerName,
      this.headerValuePrefix,
      required this.id,
      required this.layer,
      required this.name,
      this.ownerUserId,
      required this.slug,
      required this.spaceAccess,
      this.spaceId,
      required this.status,
      required this.trust,
      required this.updatedAt})
      : super._();
  @override
  IntegrationDetail rebuild(void Function(IntegrationDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  IntegrationDetailBuilder toBuilder() =>
      IntegrationDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is IntegrationDetail &&
        authKind == other.authKind &&
        catalogEntryId == other.catalogEntryId &&
        connectUrl == other.connectUrl &&
        createdAt == other.createdAt &&
        credentialMode == other.credentialMode &&
        enabled == other.enabled &&
        hasCredential == other.hasCredential &&
        headerName == other.headerName &&
        headerValuePrefix == other.headerValuePrefix &&
        id == other.id &&
        layer == other.layer &&
        name == other.name &&
        ownerUserId == other.ownerUserId &&
        slug == other.slug &&
        spaceAccess == other.spaceAccess &&
        spaceId == other.spaceId &&
        status == other.status &&
        trust == other.trust &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authKind.hashCode);
    _$hash = $jc(_$hash, catalogEntryId.hashCode);
    _$hash = $jc(_$hash, connectUrl.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, credentialMode.hashCode);
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jc(_$hash, hasCredential.hashCode);
    _$hash = $jc(_$hash, headerName.hashCode);
    _$hash = $jc(_$hash, headerValuePrefix.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, layer.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, ownerUserId.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, spaceAccess.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, trust.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'IntegrationDetail')
          ..add('authKind', authKind)
          ..add('catalogEntryId', catalogEntryId)
          ..add('connectUrl', connectUrl)
          ..add('createdAt', createdAt)
          ..add('credentialMode', credentialMode)
          ..add('enabled', enabled)
          ..add('hasCredential', hasCredential)
          ..add('headerName', headerName)
          ..add('headerValuePrefix', headerValuePrefix)
          ..add('id', id)
          ..add('layer', layer)
          ..add('name', name)
          ..add('ownerUserId', ownerUserId)
          ..add('slug', slug)
          ..add('spaceAccess', spaceAccess)
          ..add('spaceId', spaceId)
          ..add('status', status)
          ..add('trust', trust)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class IntegrationDetailBuilder
    implements Builder<IntegrationDetail, IntegrationDetailBuilder> {
  _$IntegrationDetail? _$v;

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

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  CredentialMode? _credentialMode;
  CredentialMode? get credentialMode => _$this._credentialMode;
  set credentialMode(CredentialMode? credentialMode) =>
      _$this._credentialMode = credentialMode;

  bool? _enabled;
  bool? get enabled => _$this._enabled;
  set enabled(bool? enabled) => _$this._enabled = enabled;

  bool? _hasCredential;
  bool? get hasCredential => _$this._hasCredential;
  set hasCredential(bool? hasCredential) =>
      _$this._hasCredential = hasCredential;

  String? _headerName;
  String? get headerName => _$this._headerName;
  set headerName(String? headerName) => _$this._headerName = headerName;

  String? _headerValuePrefix;
  String? get headerValuePrefix => _$this._headerValuePrefix;
  set headerValuePrefix(String? headerValuePrefix) =>
      _$this._headerValuePrefix = headerValuePrefix;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  IntegrationLayer? _layer;
  IntegrationLayer? get layer => _$this._layer;
  set layer(IntegrationLayer? layer) => _$this._layer = layer;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _ownerUserId;
  String? get ownerUserId => _$this._ownerUserId;
  set ownerUserId(String? ownerUserId) => _$this._ownerUserId = ownerUserId;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  ListBuilder<SpaceAccessEntry>? _spaceAccess;
  ListBuilder<SpaceAccessEntry> get spaceAccess =>
      _$this._spaceAccess ??= ListBuilder<SpaceAccessEntry>();
  set spaceAccess(ListBuilder<SpaceAccessEntry>? spaceAccess) =>
      _$this._spaceAccess = spaceAccess;

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  IntegrationStatus? _status;
  IntegrationStatus? get status => _$this._status;
  set status(IntegrationStatus? status) => _$this._status = status;

  IntegrationTrust? _trust;
  IntegrationTrust? get trust => _$this._trust;
  set trust(IntegrationTrust? trust) => _$this._trust = trust;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  IntegrationDetailBuilder() {
    IntegrationDetail._defaults(this);
  }

  IntegrationDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authKind = $v.authKind;
      _catalogEntryId = $v.catalogEntryId;
      _connectUrl = $v.connectUrl;
      _createdAt = $v.createdAt;
      _credentialMode = $v.credentialMode;
      _enabled = $v.enabled;
      _hasCredential = $v.hasCredential;
      _headerName = $v.headerName;
      _headerValuePrefix = $v.headerValuePrefix;
      _id = $v.id;
      _layer = $v.layer;
      _name = $v.name;
      _ownerUserId = $v.ownerUserId;
      _slug = $v.slug;
      _spaceAccess = $v.spaceAccess.toBuilder();
      _spaceId = $v.spaceId;
      _status = $v.status;
      _trust = $v.trust;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(IntegrationDetail other) {
    _$v = other as _$IntegrationDetail;
  }

  @override
  void update(void Function(IntegrationDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  IntegrationDetail build() => _build();

  _$IntegrationDetail _build() {
    _$IntegrationDetail _$result;
    try {
      _$result = _$v ??
          _$IntegrationDetail._(
            authKind: BuiltValueNullFieldError.checkNotNull(
                authKind, r'IntegrationDetail', 'authKind'),
            catalogEntryId: catalogEntryId,
            connectUrl: BuiltValueNullFieldError.checkNotNull(
                connectUrl, r'IntegrationDetail', 'connectUrl'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'IntegrationDetail', 'createdAt'),
            credentialMode: BuiltValueNullFieldError.checkNotNull(
                credentialMode, r'IntegrationDetail', 'credentialMode'),
            enabled: BuiltValueNullFieldError.checkNotNull(
                enabled, r'IntegrationDetail', 'enabled'),
            hasCredential: BuiltValueNullFieldError.checkNotNull(
                hasCredential, r'IntegrationDetail', 'hasCredential'),
            headerName: headerName,
            headerValuePrefix: headerValuePrefix,
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'IntegrationDetail', 'id'),
            layer: BuiltValueNullFieldError.checkNotNull(
                layer, r'IntegrationDetail', 'layer'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'IntegrationDetail', 'name'),
            ownerUserId: ownerUserId,
            slug: BuiltValueNullFieldError.checkNotNull(
                slug, r'IntegrationDetail', 'slug'),
            spaceAccess: spaceAccess.build(),
            spaceId: spaceId,
            status: BuiltValueNullFieldError.checkNotNull(
                status, r'IntegrationDetail', 'status'),
            trust: BuiltValueNullFieldError.checkNotNull(
                trust, r'IntegrationDetail', 'trust'),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'IntegrationDetail', 'updatedAt'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'spaceAccess';
        spaceAccess.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'IntegrationDetail', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
