// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'password_o_auth_flow.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PasswordOAuthFlow extends PasswordOAuthFlow {
  @override
  final String? refreshUrl;
  @override
  final BuiltMap<String, String>? scopes;
  @override
  final String? tokenUrl;

  factory _$PasswordOAuthFlow(
          [void Function(PasswordOAuthFlowBuilder)? updates]) =>
      (PasswordOAuthFlowBuilder()..update(updates))._build();

  _$PasswordOAuthFlow._({this.refreshUrl, this.scopes, this.tokenUrl})
      : super._();
  @override
  PasswordOAuthFlow rebuild(void Function(PasswordOAuthFlowBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PasswordOAuthFlowBuilder toBuilder() =>
      PasswordOAuthFlowBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PasswordOAuthFlow &&
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
    return (newBuiltValueToStringHelper(r'PasswordOAuthFlow')
          ..add('refreshUrl', refreshUrl)
          ..add('scopes', scopes)
          ..add('tokenUrl', tokenUrl))
        .toString();
  }
}

class PasswordOAuthFlowBuilder
    implements Builder<PasswordOAuthFlow, PasswordOAuthFlowBuilder> {
  _$PasswordOAuthFlow? _$v;

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

  PasswordOAuthFlowBuilder() {
    PasswordOAuthFlow._defaults(this);
  }

  PasswordOAuthFlowBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _refreshUrl = $v.refreshUrl;
      _scopes = $v.scopes?.toBuilder();
      _tokenUrl = $v.tokenUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PasswordOAuthFlow other) {
    _$v = other as _$PasswordOAuthFlow;
  }

  @override
  void update(void Function(PasswordOAuthFlowBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PasswordOAuthFlow build() => _build();

  _$PasswordOAuthFlow _build() {
    _$PasswordOAuthFlow _$result;
    try {
      _$result = _$v ??
          _$PasswordOAuthFlow._(
            refreshUrl: refreshUrl,
            scopes: _scopes?.build(),
            tokenUrl: tokenUrl,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        _scopes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'PasswordOAuthFlow', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
