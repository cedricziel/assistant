// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_interface.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentInterface extends AgentInterface {
  @override
  final String protocolBinding;
  @override
  final String protocolVersion;
  @override
  final String? tenant;
  @override
  final String url;

  factory _$AgentInterface([void Function(AgentInterfaceBuilder)? updates]) =>
      (AgentInterfaceBuilder()..update(updates))._build();

  _$AgentInterface._(
      {required this.protocolBinding,
      required this.protocolVersion,
      this.tenant,
      required this.url})
      : super._();
  @override
  AgentInterface rebuild(void Function(AgentInterfaceBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentInterfaceBuilder toBuilder() => AgentInterfaceBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentInterface &&
        protocolBinding == other.protocolBinding &&
        protocolVersion == other.protocolVersion &&
        tenant == other.tenant &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, protocolBinding.hashCode);
    _$hash = $jc(_$hash, protocolVersion.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentInterface')
          ..add('protocolBinding', protocolBinding)
          ..add('protocolVersion', protocolVersion)
          ..add('tenant', tenant)
          ..add('url', url))
        .toString();
  }
}

class AgentInterfaceBuilder
    implements Builder<AgentInterface, AgentInterfaceBuilder> {
  _$AgentInterface? _$v;

  String? _protocolBinding;
  String? get protocolBinding => _$this._protocolBinding;
  set protocolBinding(String? protocolBinding) =>
      _$this._protocolBinding = protocolBinding;

  String? _protocolVersion;
  String? get protocolVersion => _$this._protocolVersion;
  set protocolVersion(String? protocolVersion) =>
      _$this._protocolVersion = protocolVersion;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  AgentInterfaceBuilder() {
    AgentInterface._defaults(this);
  }

  AgentInterfaceBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _protocolBinding = $v.protocolBinding;
      _protocolVersion = $v.protocolVersion;
      _tenant = $v.tenant;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentInterface other) {
    _$v = other as _$AgentInterface;
  }

  @override
  void update(void Function(AgentInterfaceBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentInterface build() => _build();

  _$AgentInterface _build() {
    final _$result = _$v ??
        _$AgentInterface._(
          protocolBinding: BuiltValueNullFieldError.checkNotNull(
              protocolBinding, r'AgentInterface', 'protocolBinding'),
          protocolVersion: BuiltValueNullFieldError.checkNotNull(
              protocolVersion, r'AgentInterface', 'protocolVersion'),
          tenant: tenant,
          url: BuiltValueNullFieldError.checkNotNull(
              url, r'AgentInterface', 'url'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
