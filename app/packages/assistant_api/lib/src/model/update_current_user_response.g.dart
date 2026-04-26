// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_current_user_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateCurrentUserResponse extends UpdateCurrentUserResponse {
  @override
  final String? previousEmail;
  @override
  final UserDetail user;

  factory _$UpdateCurrentUserResponse(
          [void Function(UpdateCurrentUserResponseBuilder)? updates]) =>
      (UpdateCurrentUserResponseBuilder()..update(updates))._build();

  _$UpdateCurrentUserResponse._({this.previousEmail, required this.user})
      : super._();
  @override
  UpdateCurrentUserResponse rebuild(
          void Function(UpdateCurrentUserResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateCurrentUserResponseBuilder toBuilder() =>
      UpdateCurrentUserResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateCurrentUserResponse &&
        previousEmail == other.previousEmail &&
        user == other.user;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, previousEmail.hashCode);
    _$hash = $jc(_$hash, user.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateCurrentUserResponse')
          ..add('previousEmail', previousEmail)
          ..add('user', user))
        .toString();
  }
}

class UpdateCurrentUserResponseBuilder
    implements
        Builder<UpdateCurrentUserResponse, UpdateCurrentUserResponseBuilder> {
  _$UpdateCurrentUserResponse? _$v;

  String? _previousEmail;
  String? get previousEmail => _$this._previousEmail;
  set previousEmail(String? previousEmail) =>
      _$this._previousEmail = previousEmail;

  UserDetailBuilder? _user;
  UserDetailBuilder get user => _$this._user ??= UserDetailBuilder();
  set user(UserDetailBuilder? user) => _$this._user = user;

  UpdateCurrentUserResponseBuilder() {
    UpdateCurrentUserResponse._defaults(this);
  }

  UpdateCurrentUserResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _previousEmail = $v.previousEmail;
      _user = $v.user.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateCurrentUserResponse other) {
    _$v = other as _$UpdateCurrentUserResponse;
  }

  @override
  void update(void Function(UpdateCurrentUserResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateCurrentUserResponse build() => _build();

  _$UpdateCurrentUserResponse _build() {
    _$UpdateCurrentUserResponse _$result;
    try {
      _$result = _$v ??
          _$UpdateCurrentUserResponse._(
            previousEmail: previousEmail,
            user: user.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'user';
        user.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'UpdateCurrentUserResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
