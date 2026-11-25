// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credential_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CredentialResponse extends CredentialResponse {
  @override
  final bool hasCredential;
  @override
  final DateTime? updatedAt;

  factory _$CredentialResponse(
          [void Function(CredentialResponseBuilder)? updates]) =>
      (CredentialResponseBuilder()..update(updates))._build();

  _$CredentialResponse._({required this.hasCredential, this.updatedAt})
      : super._();
  @override
  CredentialResponse rebuild(
          void Function(CredentialResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CredentialResponseBuilder toBuilder() =>
      CredentialResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CredentialResponse &&
        hasCredential == other.hasCredential &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, hasCredential.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CredentialResponse')
          ..add('hasCredential', hasCredential)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class CredentialResponseBuilder
    implements Builder<CredentialResponse, CredentialResponseBuilder> {
  _$CredentialResponse? _$v;

  bool? _hasCredential;
  bool? get hasCredential => _$this._hasCredential;
  set hasCredential(bool? hasCredential) =>
      _$this._hasCredential = hasCredential;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  CredentialResponseBuilder() {
    CredentialResponse._defaults(this);
  }

  CredentialResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _hasCredential = $v.hasCredential;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CredentialResponse other) {
    _$v = other as _$CredentialResponse;
  }

  @override
  void update(void Function(CredentialResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CredentialResponse build() => _build();

  _$CredentialResponse _build() {
    final _$result = _$v ??
        _$CredentialResponse._(
          hasCredential: BuiltValueNullFieldError.checkNotNull(
              hasCredential, r'CredentialResponse', 'hasCredential'),
          updatedAt: updatedAt,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
