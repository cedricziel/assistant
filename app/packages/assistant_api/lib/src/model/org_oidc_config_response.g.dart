// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'org_oidc_config_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OrgOidcConfigResponse extends OrgOidcConfigResponse {
  @override
  final BuiltList<String> allowedEmailDomains;
  @override
  final bool autoProvision;
  @override
  final String clientId;
  @override
  final String createdAt;
  @override
  final bool hasClientSecret;
  @override
  final String issuerUrl;
  @override
  final String updatedAt;

  factory _$OrgOidcConfigResponse(
          [void Function(OrgOidcConfigResponseBuilder)? updates]) =>
      (OrgOidcConfigResponseBuilder()..update(updates))._build();

  _$OrgOidcConfigResponse._(
      {required this.allowedEmailDomains,
      required this.autoProvision,
      required this.clientId,
      required this.createdAt,
      required this.hasClientSecret,
      required this.issuerUrl,
      required this.updatedAt})
      : super._();
  @override
  OrgOidcConfigResponse rebuild(
          void Function(OrgOidcConfigResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OrgOidcConfigResponseBuilder toBuilder() =>
      OrgOidcConfigResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OrgOidcConfigResponse &&
        allowedEmailDomains == other.allowedEmailDomains &&
        autoProvision == other.autoProvision &&
        clientId == other.clientId &&
        createdAt == other.createdAt &&
        hasClientSecret == other.hasClientSecret &&
        issuerUrl == other.issuerUrl &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, allowedEmailDomains.hashCode);
    _$hash = $jc(_$hash, autoProvision.hashCode);
    _$hash = $jc(_$hash, clientId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, hasClientSecret.hashCode);
    _$hash = $jc(_$hash, issuerUrl.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OrgOidcConfigResponse')
          ..add('allowedEmailDomains', allowedEmailDomains)
          ..add('autoProvision', autoProvision)
          ..add('clientId', clientId)
          ..add('createdAt', createdAt)
          ..add('hasClientSecret', hasClientSecret)
          ..add('issuerUrl', issuerUrl)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class OrgOidcConfigResponseBuilder
    implements Builder<OrgOidcConfigResponse, OrgOidcConfigResponseBuilder> {
  _$OrgOidcConfigResponse? _$v;

  ListBuilder<String>? _allowedEmailDomains;
  ListBuilder<String> get allowedEmailDomains =>
      _$this._allowedEmailDomains ??= ListBuilder<String>();
  set allowedEmailDomains(ListBuilder<String>? allowedEmailDomains) =>
      _$this._allowedEmailDomains = allowedEmailDomains;

  bool? _autoProvision;
  bool? get autoProvision => _$this._autoProvision;
  set autoProvision(bool? autoProvision) =>
      _$this._autoProvision = autoProvision;

  String? _clientId;
  String? get clientId => _$this._clientId;
  set clientId(String? clientId) => _$this._clientId = clientId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  bool? _hasClientSecret;
  bool? get hasClientSecret => _$this._hasClientSecret;
  set hasClientSecret(bool? hasClientSecret) =>
      _$this._hasClientSecret = hasClientSecret;

  String? _issuerUrl;
  String? get issuerUrl => _$this._issuerUrl;
  set issuerUrl(String? issuerUrl) => _$this._issuerUrl = issuerUrl;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  OrgOidcConfigResponseBuilder() {
    OrgOidcConfigResponse._defaults(this);
  }

  OrgOidcConfigResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _allowedEmailDomains = $v.allowedEmailDomains.toBuilder();
      _autoProvision = $v.autoProvision;
      _clientId = $v.clientId;
      _createdAt = $v.createdAt;
      _hasClientSecret = $v.hasClientSecret;
      _issuerUrl = $v.issuerUrl;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OrgOidcConfigResponse other) {
    _$v = other as _$OrgOidcConfigResponse;
  }

  @override
  void update(void Function(OrgOidcConfigResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OrgOidcConfigResponse build() => _build();

  _$OrgOidcConfigResponse _build() {
    _$OrgOidcConfigResponse _$result;
    try {
      _$result = _$v ??
          _$OrgOidcConfigResponse._(
            allowedEmailDomains: allowedEmailDomains.build(),
            autoProvision: BuiltValueNullFieldError.checkNotNull(
                autoProvision, r'OrgOidcConfigResponse', 'autoProvision'),
            clientId: BuiltValueNullFieldError.checkNotNull(
                clientId, r'OrgOidcConfigResponse', 'clientId'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'OrgOidcConfigResponse', 'createdAt'),
            hasClientSecret: BuiltValueNullFieldError.checkNotNull(
                hasClientSecret, r'OrgOidcConfigResponse', 'hasClientSecret'),
            issuerUrl: BuiltValueNullFieldError.checkNotNull(
                issuerUrl, r'OrgOidcConfigResponse', 'issuerUrl'),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'OrgOidcConfigResponse', 'updatedAt'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'allowedEmailDomains';
        allowedEmailDomains.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'OrgOidcConfigResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
