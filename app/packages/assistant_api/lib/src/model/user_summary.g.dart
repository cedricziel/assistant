// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserSummary extends UserSummary {
  @override
  final String email;
  @override
  final String id;
  @override
  final String name;

  factory _$UserSummary([void Function(UserSummaryBuilder)? updates]) =>
      (UserSummaryBuilder()..update(updates))._build();

  _$UserSummary._({required this.email, required this.id, required this.name})
      : super._();
  @override
  UserSummary rebuild(void Function(UserSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserSummaryBuilder toBuilder() => UserSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserSummary &&
        email == other.email &&
        id == other.id &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, email.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UserSummary')
          ..add('email', email)
          ..add('id', id)
          ..add('name', name))
        .toString();
  }
}

class UserSummaryBuilder implements Builder<UserSummary, UserSummaryBuilder> {
  _$UserSummary? _$v;

  String? _email;
  String? get email => _$this._email;
  set email(String? email) => _$this._email = email;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  UserSummaryBuilder() {
    UserSummary._defaults(this);
  }

  UserSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _email = $v.email;
      _id = $v.id;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserSummary other) {
    _$v = other as _$UserSummary;
  }

  @override
  void update(void Function(UserSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserSummary build() => _build();

  _$UserSummary _build() {
    final _$result = _$v ??
        _$UserSummary._(
          email: BuiltValueNullFieldError.checkNotNull(
              email, r'UserSummary', 'email'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'UserSummary', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'UserSummary', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
