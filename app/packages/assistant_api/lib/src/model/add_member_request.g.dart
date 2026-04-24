// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'add_member_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AddMemberRequest extends AddMemberRequest {
  @override
  final String role;
  @override
  final String userId;

  factory _$AddMemberRequest(
          [void Function(AddMemberRequestBuilder)? updates]) =>
      (AddMemberRequestBuilder()..update(updates))._build();

  _$AddMemberRequest._({required this.role, required this.userId}) : super._();
  @override
  AddMemberRequest rebuild(void Function(AddMemberRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AddMemberRequestBuilder toBuilder() =>
      AddMemberRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AddMemberRequest &&
        role == other.role &&
        userId == other.userId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, userId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AddMemberRequest')
          ..add('role', role)
          ..add('userId', userId))
        .toString();
  }
}

class AddMemberRequestBuilder
    implements Builder<AddMemberRequest, AddMemberRequestBuilder> {
  _$AddMemberRequest? _$v;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  String? _userId;
  String? get userId => _$this._userId;
  set userId(String? userId) => _$this._userId = userId;

  AddMemberRequestBuilder() {
    AddMemberRequest._defaults(this);
  }

  AddMemberRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _role = $v.role;
      _userId = $v.userId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AddMemberRequest other) {
    _$v = other as _$AddMemberRequest;
  }

  @override
  void update(void Function(AddMemberRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AddMemberRequest build() => _build();

  _$AddMemberRequest _build() {
    final _$result = _$v ??
        _$AddMemberRequest._(
          role: BuiltValueNullFieldError.checkNotNull(
              role, r'AddMemberRequest', 'role'),
          userId: BuiltValueNullFieldError.checkNotNull(
              userId, r'AddMemberRequest', 'userId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
