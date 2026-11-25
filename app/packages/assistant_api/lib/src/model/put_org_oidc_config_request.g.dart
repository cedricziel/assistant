// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'put_org_oidc_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PutOrgOidcConfigRequest extends PutOrgOidcConfigRequest {
  @override
  final BuiltList<String>? allowedEmailDomains;
  @override
  final bool? autoProvision;
  @override
  final String clientId;
  @override
  final String? clientSecret;
  @override
  final String issuerUrl;

  factory _$PutOrgOidcConfigRequest(
          [void Function(PutOrgOidcConfigRequestBuilder)? updates]) =>
      (PutOrgOidcConfigRequestBuilder()..update(updates))._build();

  _$PutOrgOidcConfigRequest._(
      {this.allowedEmailDomains,
      this.autoProvision,
      required this.clientId,
      this.clientSecret,
      required this.issuerUrl})
      : super._();
  @override
  PutOrgOidcConfigRequest rebuild(
          void Function(PutOrgOidcConfigRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PutOrgOidcConfigRequestBuilder toBuilder() =>
      PutOrgOidcConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PutOrgOidcConfigRequest &&
        allowedEmailDomains == other.allowedEmailDomains &&
        autoProvision == other.autoProvision &&
        clientId == other.clientId &&
        clientSecret == other.clientSecret &&
        issuerUrl == other.issuerUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, allowedEmailDomains.hashCode);
    _$hash = $jc(_$hash, autoProvision.hashCode);
    _$hash = $jc(_$hash, clientId.hashCode);
    _$hash = $jc(_$hash, clientSecret.hashCode);
    _$hash = $jc(_$hash, issuerUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PutOrgOidcConfigRequest')
          ..add('allowedEmailDomains', allowedEmailDomains)
          ..add('autoProvision', autoProvision)
          ..add('clientId', clientId)
          ..add('clientSecret', clientSecret)
          ..add('issuerUrl', issuerUrl))
        .toString();
  }
}

class PutOrgOidcConfigRequestBuilder
    implements
        Builder<PutOrgOidcConfigRequest, PutOrgOidcConfigRequestBuilder> {
  _$PutOrgOidcConfigRequest? _$v;

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

  String? _clientSecret;
  String? get clientSecret => _$this._clientSecret;
  set clientSecret(String? clientSecret) => _$this._clientSecret = clientSecret;

  String? _issuerUrl;
  String? get issuerUrl => _$this._issuerUrl;
  set issuerUrl(String? issuerUrl) => _$this._issuerUrl = issuerUrl;

  PutOrgOidcConfigRequestBuilder() {
    PutOrgOidcConfigRequest._defaults(this);
  }

  PutOrgOidcConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _allowedEmailDomains = $v.allowedEmailDomains?.toBuilder();
      _autoProvision = $v.autoProvision;
      _clientId = $v.clientId;
      _clientSecret = $v.clientSecret;
      _issuerUrl = $v.issuerUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PutOrgOidcConfigRequest other) {
    _$v = other as _$PutOrgOidcConfigRequest;
  }

  @override
  void update(void Function(PutOrgOidcConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PutOrgOidcConfigRequest build() => _build();

  _$PutOrgOidcConfigRequest _build() {
    _$PutOrgOidcConfigRequest _$result;
    try {
      _$result = _$v ??
          _$PutOrgOidcConfigRequest._(
            allowedEmailDomains: _allowedEmailDomains?.build(),
            autoProvision: autoProvision,
            clientId: BuiltValueNullFieldError.checkNotNull(
                clientId, r'PutOrgOidcConfigRequest', 'clientId'),
            clientSecret: clientSecret,
            issuerUrl: BuiltValueNullFieldError.checkNotNull(
                issuerUrl, r'PutOrgOidcConfigRequest', 'issuerUrl'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'allowedEmailDomains';
        _allowedEmailDomains?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'PutOrgOidcConfigRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
