// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_current_user_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateCurrentUserRequest extends UpdateCurrentUserRequest {
  @override
  final String? email;
  @override
  final String? name;

  factory _$UpdateCurrentUserRequest(
          [void Function(UpdateCurrentUserRequestBuilder)? updates]) =>
      (UpdateCurrentUserRequestBuilder()..update(updates))._build();

  _$UpdateCurrentUserRequest._({this.email, this.name}) : super._();
  @override
  UpdateCurrentUserRequest rebuild(
          void Function(UpdateCurrentUserRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateCurrentUserRequestBuilder toBuilder() =>
      UpdateCurrentUserRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateCurrentUserRequest &&
        email == other.email &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, email.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateCurrentUserRequest')
          ..add('email', email)
          ..add('name', name))
        .toString();
  }
}

class UpdateCurrentUserRequestBuilder
    implements
        Builder<UpdateCurrentUserRequest, UpdateCurrentUserRequestBuilder> {
  _$UpdateCurrentUserRequest? _$v;

  String? _email;
  String? get email => _$this._email;
  set email(String? email) => _$this._email = email;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  UpdateCurrentUserRequestBuilder() {
    UpdateCurrentUserRequest._defaults(this);
  }

  UpdateCurrentUserRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _email = $v.email;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateCurrentUserRequest other) {
    _$v = other as _$UpdateCurrentUserRequest;
  }

  @override
  void update(void Function(UpdateCurrentUserRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateCurrentUserRequest build() => _build();

  _$UpdateCurrentUserRequest _build() {
    final _$result = _$v ??
        _$UpdateCurrentUserRequest._(
          email: email,
          name: name,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
