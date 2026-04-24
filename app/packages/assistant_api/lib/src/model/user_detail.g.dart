// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserDetail extends UserDetail {
  @override
  final String createdAt;
  @override
  final String email;
  @override
  final String id;
  @override
  final String name;
  @override
  final String orgId;
  @override
  final String updatedAt;

  factory _$UserDetail([void Function(UserDetailBuilder)? updates]) =>
      (UserDetailBuilder()..update(updates))._build();

  _$UserDetail._(
      {required this.createdAt,
      required this.email,
      required this.id,
      required this.name,
      required this.orgId,
      required this.updatedAt})
      : super._();
  @override
  UserDetail rebuild(void Function(UserDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserDetailBuilder toBuilder() => UserDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserDetail &&
        createdAt == other.createdAt &&
        email == other.email &&
        id == other.id &&
        name == other.name &&
        orgId == other.orgId &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, email.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, orgId.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UserDetail')
          ..add('createdAt', createdAt)
          ..add('email', email)
          ..add('id', id)
          ..add('name', name)
          ..add('orgId', orgId)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class UserDetailBuilder implements Builder<UserDetail, UserDetailBuilder> {
  _$UserDetail? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _email;
  String? get email => _$this._email;
  set email(String? email) => _$this._email = email;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _orgId;
  String? get orgId => _$this._orgId;
  set orgId(String? orgId) => _$this._orgId = orgId;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  UserDetailBuilder() {
    UserDetail._defaults(this);
  }

  UserDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _email = $v.email;
      _id = $v.id;
      _name = $v.name;
      _orgId = $v.orgId;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserDetail other) {
    _$v = other as _$UserDetail;
  }

  @override
  void update(void Function(UserDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserDetail build() => _build();

  _$UserDetail _build() {
    final _$result = _$v ??
        _$UserDetail._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'UserDetail', 'createdAt'),
          email: BuiltValueNullFieldError.checkNotNull(
              email, r'UserDetail', 'email'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'UserDetail', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'UserDetail', 'name'),
          orgId: BuiltValueNullFieldError.checkNotNull(
              orgId, r'UserDetail', 'orgId'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'UserDetail', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
