// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_member_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateMemberRequest extends UpdateMemberRequest {
  @override
  final String role;

  factory _$UpdateMemberRequest(
          [void Function(UpdateMemberRequestBuilder)? updates]) =>
      (UpdateMemberRequestBuilder()..update(updates))._build();

  _$UpdateMemberRequest._({required this.role}) : super._();
  @override
  UpdateMemberRequest rebuild(
          void Function(UpdateMemberRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateMemberRequestBuilder toBuilder() =>
      UpdateMemberRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateMemberRequest && role == other.role;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateMemberRequest')
          ..add('role', role))
        .toString();
  }
}

class UpdateMemberRequestBuilder
    implements Builder<UpdateMemberRequest, UpdateMemberRequestBuilder> {
  _$UpdateMemberRequest? _$v;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  UpdateMemberRequestBuilder() {
    UpdateMemberRequest._defaults(this);
  }

  UpdateMemberRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _role = $v.role;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateMemberRequest other) {
    _$v = other as _$UpdateMemberRequest;
  }

  @override
  void update(void Function(UpdateMemberRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateMemberRequest build() => _build();

  _$UpdateMemberRequest _build() {
    final _$result = _$v ??
        _$UpdateMemberRequest._(
          role: BuiltValueNullFieldError.checkNotNull(
              role, r'UpdateMemberRequest', 'role'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
