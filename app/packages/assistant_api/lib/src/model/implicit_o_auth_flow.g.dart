// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'implicit_o_auth_flow.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ImplicitOAuthFlow extends ImplicitOAuthFlow {
  @override
  final String? authorizationUrl;
  @override
  final String? refreshUrl;
  @override
  final BuiltMap<String, String>? scopes;

  factory _$ImplicitOAuthFlow(
          [void Function(ImplicitOAuthFlowBuilder)? updates]) =>
      (ImplicitOAuthFlowBuilder()..update(updates))._build();

  _$ImplicitOAuthFlow._({this.authorizationUrl, this.refreshUrl, this.scopes})
      : super._();
  @override
  ImplicitOAuthFlow rebuild(void Function(ImplicitOAuthFlowBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ImplicitOAuthFlowBuilder toBuilder() =>
      ImplicitOAuthFlowBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ImplicitOAuthFlow &&
        authorizationUrl == other.authorizationUrl &&
        refreshUrl == other.refreshUrl &&
        scopes == other.scopes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authorizationUrl.hashCode);
    _$hash = $jc(_$hash, refreshUrl.hashCode);
    _$hash = $jc(_$hash, scopes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ImplicitOAuthFlow')
          ..add('authorizationUrl', authorizationUrl)
          ..add('refreshUrl', refreshUrl)
          ..add('scopes', scopes))
        .toString();
  }
}

class ImplicitOAuthFlowBuilder
    implements Builder<ImplicitOAuthFlow, ImplicitOAuthFlowBuilder> {
  _$ImplicitOAuthFlow? _$v;

  String? _authorizationUrl;
  String? get authorizationUrl => _$this._authorizationUrl;
  set authorizationUrl(String? authorizationUrl) =>
      _$this._authorizationUrl = authorizationUrl;

  String? _refreshUrl;
  String? get refreshUrl => _$this._refreshUrl;
  set refreshUrl(String? refreshUrl) => _$this._refreshUrl = refreshUrl;

  MapBuilder<String, String>? _scopes;
  MapBuilder<String, String> get scopes =>
      _$this._scopes ??= MapBuilder<String, String>();
  set scopes(MapBuilder<String, String>? scopes) => _$this._scopes = scopes;

  ImplicitOAuthFlowBuilder() {
    ImplicitOAuthFlow._defaults(this);
  }

  ImplicitOAuthFlowBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authorizationUrl = $v.authorizationUrl;
      _refreshUrl = $v.refreshUrl;
      _scopes = $v.scopes?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ImplicitOAuthFlow other) {
    _$v = other as _$ImplicitOAuthFlow;
  }

  @override
  void update(void Function(ImplicitOAuthFlowBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ImplicitOAuthFlow build() => _build();

  _$ImplicitOAuthFlow _build() {
    _$ImplicitOAuthFlow _$result;
    try {
      _$result = _$v ??
          _$ImplicitOAuthFlow._(
            authorizationUrl: authorizationUrl,
            refreshUrl: refreshUrl,
            scopes: _scopes?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'scopes';
        _scopes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ImplicitOAuthFlow', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
