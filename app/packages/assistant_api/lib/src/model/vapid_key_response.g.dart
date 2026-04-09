// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vapid_key_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$VapidKeyResponse extends VapidKeyResponse {
  @override
  final String publicKey;

  factory _$VapidKeyResponse(
          [void Function(VapidKeyResponseBuilder)? updates]) =>
      (VapidKeyResponseBuilder()..update(updates))._build();

  _$VapidKeyResponse._({required this.publicKey}) : super._();
  @override
  VapidKeyResponse rebuild(void Function(VapidKeyResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  VapidKeyResponseBuilder toBuilder() =>
      VapidKeyResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is VapidKeyResponse && publicKey == other.publicKey;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, publicKey.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'VapidKeyResponse')
          ..add('publicKey', publicKey))
        .toString();
  }
}

class VapidKeyResponseBuilder
    implements Builder<VapidKeyResponse, VapidKeyResponseBuilder> {
  _$VapidKeyResponse? _$v;

  String? _publicKey;
  String? get publicKey => _$this._publicKey;
  set publicKey(String? publicKey) => _$this._publicKey = publicKey;

  VapidKeyResponseBuilder() {
    VapidKeyResponse._defaults(this);
  }

  VapidKeyResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _publicKey = $v.publicKey;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(VapidKeyResponse other) {
    _$v = other as _$VapidKeyResponse;
  }

  @override
  void update(void Function(VapidKeyResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  VapidKeyResponse build() => _build();

  _$VapidKeyResponse _build() {
    final _$result = _$v ??
        _$VapidKeyResponse._(
          publicKey: BuiltValueNullFieldError.checkNotNull(
              publicKey, r'VapidKeyResponse', 'publicKey'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
