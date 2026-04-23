// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'o_auth_error_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OAuthErrorResponse extends OAuthErrorResponse {
  @override
  final String error;
  @override
  final String errorDescription;

  factory _$OAuthErrorResponse(
          [void Function(OAuthErrorResponseBuilder)? updates]) =>
      (OAuthErrorResponseBuilder()..update(updates))._build();

  _$OAuthErrorResponse._({required this.error, required this.errorDescription})
      : super._();
  @override
  OAuthErrorResponse rebuild(
          void Function(OAuthErrorResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OAuthErrorResponseBuilder toBuilder() =>
      OAuthErrorResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OAuthErrorResponse &&
        error == other.error &&
        errorDescription == other.errorDescription;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, error.hashCode);
    _$hash = $jc(_$hash, errorDescription.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OAuthErrorResponse')
          ..add('error', error)
          ..add('errorDescription', errorDescription))
        .toString();
  }
}

class OAuthErrorResponseBuilder
    implements Builder<OAuthErrorResponse, OAuthErrorResponseBuilder> {
  _$OAuthErrorResponse? _$v;

  String? _error;
  String? get error => _$this._error;
  set error(String? error) => _$this._error = error;

  String? _errorDescription;
  String? get errorDescription => _$this._errorDescription;
  set errorDescription(String? errorDescription) =>
      _$this._errorDescription = errorDescription;

  OAuthErrorResponseBuilder() {
    OAuthErrorResponse._defaults(this);
  }

  OAuthErrorResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _error = $v.error;
      _errorDescription = $v.errorDescription;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OAuthErrorResponse other) {
    _$v = other as _$OAuthErrorResponse;
  }

  @override
  void update(void Function(OAuthErrorResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OAuthErrorResponse build() => _build();

  _$OAuthErrorResponse _build() {
    final _$result = _$v ??
        _$OAuthErrorResponse._(
          error: BuiltValueNullFieldError.checkNotNull(
              error, r'OAuthErrorResponse', 'error'),
          errorDescription: BuiltValueNullFieldError.checkNotNull(
              errorDescription, r'OAuthErrorResponse', 'errorDescription'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
