// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_provider.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentProvider extends AgentProvider {
  @override
  final String organization;
  @override
  final String url;

  factory _$AgentProvider([void Function(AgentProviderBuilder)? updates]) =>
      (AgentProviderBuilder()..update(updates))._build();

  _$AgentProvider._({required this.organization, required this.url})
      : super._();
  @override
  AgentProvider rebuild(void Function(AgentProviderBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentProviderBuilder toBuilder() => AgentProviderBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentProvider &&
        organization == other.organization &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, organization.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentProvider')
          ..add('organization', organization)
          ..add('url', url))
        .toString();
  }
}

class AgentProviderBuilder
    implements Builder<AgentProvider, AgentProviderBuilder> {
  _$AgentProvider? _$v;

  String? _organization;
  String? get organization => _$this._organization;
  set organization(String? organization) => _$this._organization = organization;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  AgentProviderBuilder() {
    AgentProvider._defaults(this);
  }

  AgentProviderBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _organization = $v.organization;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentProvider other) {
    _$v = other as _$AgentProvider;
  }

  @override
  void update(void Function(AgentProviderBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentProvider build() => _build();

  _$AgentProvider _build() {
    final _$result = _$v ??
        _$AgentProvider._(
          organization: BuiltValueNullFieldError.checkNotNull(
              organization, r'AgentProvider', 'organization'),
          url: BuiltValueNullFieldError.checkNotNull(
              url, r'AgentProvider', 'url'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
