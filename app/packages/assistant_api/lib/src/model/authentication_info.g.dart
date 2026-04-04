// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'authentication_info.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AuthenticationInfo extends AuthenticationInfo {
  @override
  final String? credentials;
  @override
  final String scheme;

  factory _$AuthenticationInfo(
          [void Function(AuthenticationInfoBuilder)? updates]) =>
      (AuthenticationInfoBuilder()..update(updates))._build();

  _$AuthenticationInfo._({this.credentials, required this.scheme}) : super._();
  @override
  AuthenticationInfo rebuild(
          void Function(AuthenticationInfoBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AuthenticationInfoBuilder toBuilder() =>
      AuthenticationInfoBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AuthenticationInfo &&
        credentials == other.credentials &&
        scheme == other.scheme;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, credentials.hashCode);
    _$hash = $jc(_$hash, scheme.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AuthenticationInfo')
          ..add('credentials', credentials)
          ..add('scheme', scheme))
        .toString();
  }
}

class AuthenticationInfoBuilder
    implements Builder<AuthenticationInfo, AuthenticationInfoBuilder> {
  _$AuthenticationInfo? _$v;

  String? _credentials;
  String? get credentials => _$this._credentials;
  set credentials(String? credentials) => _$this._credentials = credentials;

  String? _scheme;
  String? get scheme => _$this._scheme;
  set scheme(String? scheme) => _$this._scheme = scheme;

  AuthenticationInfoBuilder() {
    AuthenticationInfo._defaults(this);
  }

  AuthenticationInfoBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _credentials = $v.credentials;
      _scheme = $v.scheme;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AuthenticationInfo other) {
    _$v = other as _$AuthenticationInfo;
  }

  @override
  void update(void Function(AuthenticationInfoBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AuthenticationInfo build() => _build();

  _$AuthenticationInfo _build() {
    final _$result = _$v ??
        _$AuthenticationInfo._(
          credentials: credentials,
          scheme: BuiltValueNullFieldError.checkNotNull(
              scheme, r'AuthenticationInfo', 'scheme'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
