// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'device_code_o_auth_flow.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeviceCodeOAuthFlow extends DeviceCodeOAuthFlow {
  @override
  final String deviceAuthorizationUrl;
  @override
  final String? refreshUrl;
  @override
  final BuiltMap<String, String> scopes;
  @override
  final String tokenUrl;

  factory _$DeviceCodeOAuthFlow(
          [void Function(DeviceCodeOAuthFlowBuilder)? updates]) =>
      (DeviceCodeOAuthFlowBuilder()..update(updates))._build();

  _$DeviceCodeOAuthFlow._(
      {required this.deviceAuthorizationUrl,
      this.refreshUrl,
      required this.scopes,
      required this.tokenUrl})
      : super._();
  @override
  DeviceCodeOAuthFlow rebuild(
          void Function(DeviceCodeOAuthFlowBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  DeviceCodeOAuthFlowBuilder toBuilder() =>
      DeviceCodeOAuthFlowBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeviceCodeOAuthFlow &&
        deviceAuthorizationUrl == other.deviceAuthorizationUrl &&
        refreshUrl == other.refreshUrl &&
        scopes == other.scopes &&
        tokenUrl == other.tokenUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, deviceAuthorizationUrl.hashCode);
    _$hash = $jc(_$hash, refreshUrl.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jc(_$hash, tokenUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeviceCodeOAuthFlow')
          ..add('deviceAuthorizationUrl', deviceAuthorizationUrl)
          ..add('refreshUrl', refreshUrl)
          ..add('scopes', scopes)
          ..add('tokenUrl', tokenUrl))
        .toString();
  }
}

class DeviceCodeOAuthFlowBuilder
    implements Builder<DeviceCodeOAuthFlow, DeviceCodeOAuthFlowBuilder> {
  _$DeviceCodeOAuthFlow? _$v;

  String? _deviceAuthorizationUrl;
  String? get deviceAuthorizationUrl => _$this._deviceAuthorizationUrl;
  set deviceAuthorizationUrl(String? deviceAuthorizationUrl) =>
      _$this._deviceAuthorizationUrl = deviceAuthorizationUrl;

  String? _refreshUrl;
  String? get refreshUrl => _$this._refreshUrl;
  set refreshUrl(String? refreshUrl) => _$this._refreshUrl = refreshUrl;

  MapBuilder<String, String>? _scopes;
  MapBuilder<String, String> get scopes =>
      _$this._scopes ??= MapBuilder<String, String>();
  set scopes(MapBuilder<String, String>? scopes) => _$this._scopes = scopes;

  String? _tokenUrl;
  String? get tokenUrl => _$this._tokenUrl;
  set tokenUrl(String? tokenUrl) => _$this._tokenUrl = tokenUrl;

  DeviceCodeOAuthFlowBuilder() {
    DeviceCodeOAuthFlow._defaults(this);
  }

  DeviceCodeOAuthFlowBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _deviceAuthorizationUrl = $v.deviceAuthorizationUrl;
      _refreshUrl = $v.refreshUrl;
      _scopes = $v.scopes.toBuilder();
      _tokenUrl = $v.tokenUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeviceCodeOAuthFlow other) {
    _$v = other as _$DeviceCodeOAuthFlow;
  }

  @override
  void update(void Function(DeviceCodeOAuthFlowBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeviceCodeOAuthFlow build() => _build();

  _$DeviceCodeOAuthFlow _build() {
    _$DeviceCodeOAuthFlow _$result;
    try {
      _$result = _$v ??
          _$DeviceCodeOAuthFlow._(
            deviceAuthorizationUrl: BuiltValueNullFieldError.checkNotNull(
                deviceAuthorizationUrl,
                r'DeviceCodeOAuthFlow',
                'deviceAuthorizationUrl'),
            refreshUrl: refreshUrl,
            scopes: scopes.build(),
            tokenUrl: BuiltValueNullFieldError.checkNotNull(
                tokenUrl, r'DeviceCodeOAuthFlow', 'tokenUrl'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        scopes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'DeviceCodeOAuthFlow', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
