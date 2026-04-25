// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'binding_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$BindingResponse extends BindingResponse {
  @override
  final String createdAt;
  @override
  final String id;
  @override
  final String interfaceInstanceId;
  @override
  final String personaId;

  factory _$BindingResponse([void Function(BindingResponseBuilder)? updates]) =>
      (BindingResponseBuilder()..update(updates))._build();

  _$BindingResponse._(
      {required this.createdAt,
      required this.id,
      required this.interfaceInstanceId,
      required this.personaId})
      : super._();
  @override
  BindingResponse rebuild(void Function(BindingResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  BindingResponseBuilder toBuilder() => BindingResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is BindingResponse &&
        createdAt == other.createdAt &&
        id == other.id &&
        interfaceInstanceId == other.interfaceInstanceId &&
        personaId == other.personaId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, interfaceInstanceId.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'BindingResponse')
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('interfaceInstanceId', interfaceInstanceId)
          ..add('personaId', personaId))
        .toString();
  }
}

class BindingResponseBuilder
    implements Builder<BindingResponse, BindingResponseBuilder> {
  _$BindingResponse? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _interfaceInstanceId;
  String? get interfaceInstanceId => _$this._interfaceInstanceId;
  set interfaceInstanceId(String? interfaceInstanceId) =>
      _$this._interfaceInstanceId = interfaceInstanceId;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  BindingResponseBuilder() {
    BindingResponse._defaults(this);
  }

  BindingResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _id = $v.id;
      _interfaceInstanceId = $v.interfaceInstanceId;
      _personaId = $v.personaId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(BindingResponse other) {
    _$v = other as _$BindingResponse;
  }

  @override
  void update(void Function(BindingResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  BindingResponse build() => _build();

  _$BindingResponse _build() {
    final _$result = _$v ??
        _$BindingResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'BindingResponse', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'BindingResponse', 'id'),
          interfaceInstanceId: BuiltValueNullFieldError.checkNotNull(
              interfaceInstanceId, r'BindingResponse', 'interfaceInstanceId'),
          personaId: BuiltValueNullFieldError.checkNotNull(
              personaId, r'BindingResponse', 'personaId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
