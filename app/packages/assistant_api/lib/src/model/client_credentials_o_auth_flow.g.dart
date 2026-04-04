// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'client_credentials_o_auth_flow.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ClientCredentialsOAuthFlow extends ClientCredentialsOAuthFlow {
  @override
  final String? refreshUrl;
  @override
  final BuiltMap<String, String> scopes;
  @override
  final String tokenUrl;

  factory _$ClientCredentialsOAuthFlow(
          [void Function(ClientCredentialsOAuthFlowBuilder)? updates]) =>
      (ClientCredentialsOAuthFlowBuilder()..update(updates))._build();

  _$ClientCredentialsOAuthFlow._(
      {this.refreshUrl, required this.scopes, required this.tokenUrl})
      : super._();
  @override
  ClientCredentialsOAuthFlow rebuild(
          void Function(ClientCredentialsOAuthFlowBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ClientCredentialsOAuthFlowBuilder toBuilder() =>
      ClientCredentialsOAuthFlowBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ClientCredentialsOAuthFlow &&
        refreshUrl == other.refreshUrl &&
        scopes == other.scopes &&
        tokenUrl == other.tokenUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, refreshUrl.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jc(_$hash, tokenUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ClientCredentialsOAuthFlow')
          ..add('refreshUrl', refreshUrl)
          ..add('scopes', scopes)
          ..add('tokenUrl', tokenUrl))
        .toString();
  }
}

class ClientCredentialsOAuthFlowBuilder
    implements
        Builder<ClientCredentialsOAuthFlow, ClientCredentialsOAuthFlowBuilder> {
  _$ClientCredentialsOAuthFlow? _$v;

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

  ClientCredentialsOAuthFlowBuilder() {
    ClientCredentialsOAuthFlow._defaults(this);
  }

  ClientCredentialsOAuthFlowBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _refreshUrl = $v.refreshUrl;
      _scopes = $v.scopes.toBuilder();
      _tokenUrl = $v.tokenUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ClientCredentialsOAuthFlow other) {
    _$v = other as _$ClientCredentialsOAuthFlow;
  }

  @override
  void update(void Function(ClientCredentialsOAuthFlowBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ClientCredentialsOAuthFlow build() => _build();

  _$ClientCredentialsOAuthFlow _build() {
    _$ClientCredentialsOAuthFlow _$result;
    try {
      _$result = _$v ??
          _$ClientCredentialsOAuthFlow._(
            refreshUrl: refreshUrl,
            scopes: scopes.build(),
            tokenUrl: BuiltValueNullFieldError.checkNotNull(
                tokenUrl, r'ClientCredentialsOAuthFlow', 'tokenUrl'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        scopes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ClientCredentialsOAuthFlow', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
