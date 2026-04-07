// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_persona_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreatePersonaRequest extends CreatePersonaRequest {
  @override
  final String id;
  @override
  final String name;

  factory _$CreatePersonaRequest(
          [void Function(CreatePersonaRequestBuilder)? updates]) =>
      (CreatePersonaRequestBuilder()..update(updates))._build();

  _$CreatePersonaRequest._({required this.id, required this.name}) : super._();
  @override
  CreatePersonaRequest rebuild(
          void Function(CreatePersonaRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreatePersonaRequestBuilder toBuilder() =>
      CreatePersonaRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreatePersonaRequest &&
        id == other.id &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreatePersonaRequest')
          ..add('id', id)
          ..add('name', name))
        .toString();
  }
}

class CreatePersonaRequestBuilder
    implements Builder<CreatePersonaRequest, CreatePersonaRequestBuilder> {
  _$CreatePersonaRequest? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreatePersonaRequestBuilder() {
    CreatePersonaRequest._defaults(this);
  }

  CreatePersonaRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreatePersonaRequest other) {
    _$v = other as _$CreatePersonaRequest;
  }

  @override
  void update(void Function(CreatePersonaRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreatePersonaRequest build() => _build();

  _$CreatePersonaRequest _build() {
    final _$result = _$v ??
        _$CreatePersonaRequest._(
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'CreatePersonaRequest', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CreatePersonaRequest', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
