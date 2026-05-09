// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'error_body.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ErrorBody extends ErrorBody {
  @override
  final String error;

  factory _$ErrorBody([void Function(ErrorBodyBuilder)? updates]) =>
      (ErrorBodyBuilder()..update(updates))._build();

  _$ErrorBody._({required this.error}) : super._();
  @override
  ErrorBody rebuild(void Function(ErrorBodyBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ErrorBodyBuilder toBuilder() => ErrorBodyBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ErrorBody && error == other.error;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, error.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ErrorBody')..add('error', error))
        .toString();
  }
}

class ErrorBodyBuilder implements Builder<ErrorBody, ErrorBodyBuilder> {
  _$ErrorBody? _$v;

  String? _error;
  String? get error => _$this._error;
  set error(String? error) => _$this._error = error;

  ErrorBodyBuilder() {
    ErrorBody._defaults(this);
  }

  ErrorBodyBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _error = $v.error;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ErrorBody other) {
    _$v = other as _$ErrorBody;
  }

  @override
  void update(void Function(ErrorBodyBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ErrorBody build() => _build();

  _$ErrorBody _build() {
    final _$result = _$v ??
        _$ErrorBody._(
          error: BuiltValueNullFieldError.checkNotNull(
              error, r'ErrorBody', 'error'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
