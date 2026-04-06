// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rotate_secret_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RotateSecretResponse extends RotateSecretResponse {
  @override
  final String secret;

  factory _$RotateSecretResponse(
          [void Function(RotateSecretResponseBuilder)? updates]) =>
      (RotateSecretResponseBuilder()..update(updates))._build();

  _$RotateSecretResponse._({required this.secret}) : super._();
  @override
  RotateSecretResponse rebuild(
          void Function(RotateSecretResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RotateSecretResponseBuilder toBuilder() =>
      RotateSecretResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RotateSecretResponse && secret == other.secret;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, secret.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RotateSecretResponse')
          ..add('secret', secret))
        .toString();
  }
}

class RotateSecretResponseBuilder
    implements Builder<RotateSecretResponse, RotateSecretResponseBuilder> {
  _$RotateSecretResponse? _$v;

  String? _secret;
  String? get secret => _$this._secret;
  set secret(String? secret) => _$this._secret = secret;

  RotateSecretResponseBuilder() {
    RotateSecretResponse._defaults(this);
  }

  RotateSecretResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _secret = $v.secret;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RotateSecretResponse other) {
    _$v = other as _$RotateSecretResponse;
  }

  @override
  void update(void Function(RotateSecretResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RotateSecretResponse build() => _build();

  _$RotateSecretResponse _build() {
    final _$result = _$v ??
        _$RotateSecretResponse._(
          secret: BuiltValueNullFieldError.checkNotNull(
              secret, r'RotateSecretResponse', 'secret'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
