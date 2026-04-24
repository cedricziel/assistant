// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_org_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateOrgRequest extends UpdateOrgRequest {
  @override
  final String? authMode;
  @override
  final String? name;

  factory _$UpdateOrgRequest(
          [void Function(UpdateOrgRequestBuilder)? updates]) =>
      (UpdateOrgRequestBuilder()..update(updates))._build();

  _$UpdateOrgRequest._({this.authMode, this.name}) : super._();
  @override
  UpdateOrgRequest rebuild(void Function(UpdateOrgRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateOrgRequestBuilder toBuilder() =>
      UpdateOrgRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateOrgRequest &&
        authMode == other.authMode &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authMode.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateOrgRequest')
          ..add('authMode', authMode)
          ..add('name', name))
        .toString();
  }
}

class UpdateOrgRequestBuilder
    implements Builder<UpdateOrgRequest, UpdateOrgRequestBuilder> {
  _$UpdateOrgRequest? _$v;

  String? _authMode;
  String? get authMode => _$this._authMode;
  set authMode(String? authMode) => _$this._authMode = authMode;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  UpdateOrgRequestBuilder() {
    UpdateOrgRequest._defaults(this);
  }

  UpdateOrgRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authMode = $v.authMode;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateOrgRequest other) {
    _$v = other as _$UpdateOrgRequest;
  }

  @override
  void update(void Function(UpdateOrgRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateOrgRequest build() => _build();

  _$UpdateOrgRequest _build() {
    final _$result = _$v ??
        _$UpdateOrgRequest._(
          authMode: authMode,
          name: name,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
