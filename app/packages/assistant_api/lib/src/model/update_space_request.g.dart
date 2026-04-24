// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_space_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateSpaceRequest extends UpdateSpaceRequest {
  @override
  final String? name;

  factory _$UpdateSpaceRequest(
          [void Function(UpdateSpaceRequestBuilder)? updates]) =>
      (UpdateSpaceRequestBuilder()..update(updates))._build();

  _$UpdateSpaceRequest._({this.name}) : super._();
  @override
  UpdateSpaceRequest rebuild(
          void Function(UpdateSpaceRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateSpaceRequestBuilder toBuilder() =>
      UpdateSpaceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateSpaceRequest && name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateSpaceRequest')
          ..add('name', name))
        .toString();
  }
}

class UpdateSpaceRequestBuilder
    implements Builder<UpdateSpaceRequest, UpdateSpaceRequestBuilder> {
  _$UpdateSpaceRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  UpdateSpaceRequestBuilder() {
    UpdateSpaceRequest._defaults(this);
  }

  UpdateSpaceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateSpaceRequest other) {
    _$v = other as _$UpdateSpaceRequest;
  }

  @override
  void update(void Function(UpdateSpaceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateSpaceRequest build() => _build();

  _$UpdateSpaceRequest _build() {
    final _$result = _$v ??
        _$UpdateSpaceRequest._(
          name: name,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
