// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_integration_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateIntegrationRequest extends UpdateIntegrationRequest {
  @override
  final String? connectUrl;
  @override
  final bool? enabled;
  @override
  final String? headerName;
  @override
  final String? headerValuePrefix;
  @override
  final String? name;
  @override
  final String? slug;
  @override
  final IntegrationTrust? trust;

  factory _$UpdateIntegrationRequest(
          [void Function(UpdateIntegrationRequestBuilder)? updates]) =>
      (UpdateIntegrationRequestBuilder()..update(updates))._build();

  _$UpdateIntegrationRequest._(
      {this.connectUrl,
      this.enabled,
      this.headerName,
      this.headerValuePrefix,
      this.name,
      this.slug,
      this.trust})
      : super._();
  @override
  UpdateIntegrationRequest rebuild(
          void Function(UpdateIntegrationRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateIntegrationRequestBuilder toBuilder() =>
      UpdateIntegrationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateIntegrationRequest &&
        connectUrl == other.connectUrl &&
        enabled == other.enabled &&
        headerName == other.headerName &&
        headerValuePrefix == other.headerValuePrefix &&
        name == other.name &&
        slug == other.slug &&
        trust == other.trust;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, connectUrl.hashCode);
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jc(_$hash, headerName.hashCode);
    _$hash = $jc(_$hash, headerValuePrefix.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, trust.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateIntegrationRequest')
          ..add('connectUrl', connectUrl)
          ..add('enabled', enabled)
          ..add('headerName', headerName)
          ..add('headerValuePrefix', headerValuePrefix)
          ..add('name', name)
          ..add('slug', slug)
          ..add('trust', trust))
        .toString();
  }
}

class UpdateIntegrationRequestBuilder
    implements
        Builder<UpdateIntegrationRequest, UpdateIntegrationRequestBuilder> {
  _$UpdateIntegrationRequest? _$v;

  String? _connectUrl;
  String? get connectUrl => _$this._connectUrl;
  set connectUrl(String? connectUrl) => _$this._connectUrl = connectUrl;

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

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  IntegrationTrust? _trust;
  IntegrationTrust? get trust => _$this._trust;
  set trust(IntegrationTrust? trust) => _$this._trust = trust;

  UpdateIntegrationRequestBuilder() {
    UpdateIntegrationRequest._defaults(this);
  }

  UpdateIntegrationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _connectUrl = $v.connectUrl;
      _enabled = $v.enabled;
      _headerName = $v.headerName;
      _headerValuePrefix = $v.headerValuePrefix;
      _name = $v.name;
      _slug = $v.slug;
      _trust = $v.trust;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateIntegrationRequest other) {
    _$v = other as _$UpdateIntegrationRequest;
  }

  @override
  void update(void Function(UpdateIntegrationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateIntegrationRequest build() => _build();

  _$UpdateIntegrationRequest _build() {
    final _$result = _$v ??
        _$UpdateIntegrationRequest._(
          connectUrl: connectUrl,
          enabled: enabled,
          headerName: headerName,
          headerValuePrefix: headerValuePrefix,
          name: name,
          slug: slug,
          trust: trust,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
