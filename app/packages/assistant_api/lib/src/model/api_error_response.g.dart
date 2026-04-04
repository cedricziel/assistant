// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_error_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApiErrorResponse extends ApiErrorResponse {
  @override
  final int code;
  @override
  final String message;

  factory _$ApiErrorResponse(
          [void Function(ApiErrorResponseBuilder)? updates]) =>
      (ApiErrorResponseBuilder()..update(updates))._build();

  _$ApiErrorResponse._({required this.code, required this.message}) : super._();
  @override
  ApiErrorResponse rebuild(void Function(ApiErrorResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApiErrorResponseBuilder toBuilder() =>
      ApiErrorResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApiErrorResponse &&
        code == other.code &&
        message == other.message;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, code.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApiErrorResponse')
          ..add('code', code)
          ..add('message', message))
        .toString();
  }
}

class ApiErrorResponseBuilder
    implements Builder<ApiErrorResponse, ApiErrorResponseBuilder> {
  _$ApiErrorResponse? _$v;

  int? _code;
  int? get code => _$this._code;
  set code(int? code) => _$this._code = code;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  ApiErrorResponseBuilder() {
    ApiErrorResponse._defaults(this);
  }

  ApiErrorResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _code = $v.code;
      _message = $v.message;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApiErrorResponse other) {
    _$v = other as _$ApiErrorResponse;
  }

  @override
  void update(void Function(ApiErrorResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApiErrorResponse build() => _build();

  _$ApiErrorResponse _build() {
    final _$result = _$v ??
        _$ApiErrorResponse._(
          code: BuiltValueNullFieldError.checkNotNull(
              code, r'ApiErrorResponse', 'code'),
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'ApiErrorResponse', 'message'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
