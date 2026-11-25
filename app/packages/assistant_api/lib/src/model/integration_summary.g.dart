// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$IntegrationSummary extends IntegrationSummary {
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
  final String id;
  @override
  final DateTime? lastConnectedAt;
  @override
  final IntegrationLayer layer;
  @override
  final String name;
  @override
  final String? ownerUserId;
  @override
  final String slug;
  @override
  final String? spaceId;
  @override
  final IntegrationStatus status;
  @override
  final int? toolCount;
  @override
  final IntegrationTrust trust;
  @override
  final DateTime updatedAt;

  factory _$IntegrationSummary(
          [void Function(IntegrationSummaryBuilder)? updates]) =>
      (IntegrationSummaryBuilder()..update(updates))._build();

  _$IntegrationSummary._(
      {required this.authKind,
      this.catalogEntryId,
      required this.connectUrl,
      required this.createdAt,
      required this.credentialMode,
      required this.enabled,
      required this.hasCredential,
      required this.id,
      this.lastConnectedAt,
      required this.layer,
      required this.name,
      this.ownerUserId,
      required this.slug,
      this.spaceId,
      required this.status,
      this.toolCount,
      required this.trust,
      required this.updatedAt})
      : super._();
  @override
  IntegrationSummary rebuild(
          void Function(IntegrationSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  IntegrationSummaryBuilder toBuilder() =>
      IntegrationSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is IntegrationSummary &&
        authKind == other.authKind &&
        catalogEntryId == other.catalogEntryId &&
        connectUrl == other.connectUrl &&
        createdAt == other.createdAt &&
        credentialMode == other.credentialMode &&
        enabled == other.enabled &&
        hasCredential == other.hasCredential &&
        id == other.id &&
        lastConnectedAt == other.lastConnectedAt &&
        layer == other.layer &&
        name == other.name &&
        ownerUserId == other.ownerUserId &&
        slug == other.slug &&
        spaceId == other.spaceId &&
        status == other.status &&
        toolCount == other.toolCount &&
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
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, lastConnectedAt.hashCode);
    _$hash = $jc(_$hash, layer.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, ownerUserId.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, toolCount.hashCode);
    _$hash = $jc(_$hash, trust.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'IntegrationSummary')
          ..add('authKind', authKind)
          ..add('catalogEntryId', catalogEntryId)
          ..add('connectUrl', connectUrl)
          ..add('createdAt', createdAt)
          ..add('credentialMode', credentialMode)
          ..add('enabled', enabled)
          ..add('hasCredential', hasCredential)
          ..add('id', id)
          ..add('lastConnectedAt', lastConnectedAt)
          ..add('layer', layer)
          ..add('name', name)
          ..add('ownerUserId', ownerUserId)
          ..add('slug', slug)
          ..add('spaceId', spaceId)
          ..add('status', status)
          ..add('toolCount', toolCount)
          ..add('trust', trust)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class IntegrationSummaryBuilder
    implements Builder<IntegrationSummary, IntegrationSummaryBuilder> {
  _$IntegrationSummary? _$v;

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

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  DateTime? _lastConnectedAt;
  DateTime? get lastConnectedAt => _$this._lastConnectedAt;
  set lastConnectedAt(DateTime? lastConnectedAt) =>
      _$this._lastConnectedAt = lastConnectedAt;

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

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  IntegrationStatus? _status;
  IntegrationStatus? get status => _$this._status;
  set status(IntegrationStatus? status) => _$this._status = status;

  int? _toolCount;
  int? get toolCount => _$this._toolCount;
  set toolCount(int? toolCount) => _$this._toolCount = toolCount;

  IntegrationTrust? _trust;
  IntegrationTrust? get trust => _$this._trust;
  set trust(IntegrationTrust? trust) => _$this._trust = trust;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  IntegrationSummaryBuilder() {
    IntegrationSummary._defaults(this);
  }

  IntegrationSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authKind = $v.authKind;
      _catalogEntryId = $v.catalogEntryId;
      _connectUrl = $v.connectUrl;
      _createdAt = $v.createdAt;
      _credentialMode = $v.credentialMode;
      _enabled = $v.enabled;
      _hasCredential = $v.hasCredential;
      _id = $v.id;
      _lastConnectedAt = $v.lastConnectedAt;
      _layer = $v.layer;
      _name = $v.name;
      _ownerUserId = $v.ownerUserId;
      _slug = $v.slug;
      _spaceId = $v.spaceId;
      _status = $v.status;
      _toolCount = $v.toolCount;
      _trust = $v.trust;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(IntegrationSummary other) {
    _$v = other as _$IntegrationSummary;
  }

  @override
  void update(void Function(IntegrationSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  IntegrationSummary build() => _build();

  _$IntegrationSummary _build() {
    final _$result = _$v ??
        _$IntegrationSummary._(
          authKind: BuiltValueNullFieldError.checkNotNull(
              authKind, r'IntegrationSummary', 'authKind'),
          catalogEntryId: catalogEntryId,
          connectUrl: BuiltValueNullFieldError.checkNotNull(
              connectUrl, r'IntegrationSummary', 'connectUrl'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'IntegrationSummary', 'createdAt'),
          credentialMode: BuiltValueNullFieldError.checkNotNull(
              credentialMode, r'IntegrationSummary', 'credentialMode'),
          enabled: BuiltValueNullFieldError.checkNotNull(
              enabled, r'IntegrationSummary', 'enabled'),
          hasCredential: BuiltValueNullFieldError.checkNotNull(
              hasCredential, r'IntegrationSummary', 'hasCredential'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'IntegrationSummary', 'id'),
          lastConnectedAt: lastConnectedAt,
          layer: BuiltValueNullFieldError.checkNotNull(
              layer, r'IntegrationSummary', 'layer'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'IntegrationSummary', 'name'),
          ownerUserId: ownerUserId,
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'IntegrationSummary', 'slug'),
          spaceId: spaceId,
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'IntegrationSummary', 'status'),
          toolCount: toolCount,
          trust: BuiltValueNullFieldError.checkNotNull(
              trust, r'IntegrationSummary', 'trust'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'IntegrationSummary', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
