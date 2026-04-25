// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'interface_instance_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$InterfaceInstanceResponse extends InterfaceInstanceResponse {
  @override
  final JsonObject? config;
  @override
  final String createdAt;
  @override
  final String id;
  @override
  final String interfaceType;
  @override
  final String spaceId;

  factory _$InterfaceInstanceResponse(
          [void Function(InterfaceInstanceResponseBuilder)? updates]) =>
      (InterfaceInstanceResponseBuilder()..update(updates))._build();

  _$InterfaceInstanceResponse._(
      {this.config,
      required this.createdAt,
      required this.id,
      required this.interfaceType,
      required this.spaceId})
      : super._();
  @override
  InterfaceInstanceResponse rebuild(
          void Function(InterfaceInstanceResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  InterfaceInstanceResponseBuilder toBuilder() =>
      InterfaceInstanceResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is InterfaceInstanceResponse &&
        config == other.config &&
        createdAt == other.createdAt &&
        id == other.id &&
        interfaceType == other.interfaceType &&
        spaceId == other.spaceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, config.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, interfaceType.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'InterfaceInstanceResponse')
          ..add('config', config)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('interfaceType', interfaceType)
          ..add('spaceId', spaceId))
        .toString();
  }
}

class InterfaceInstanceResponseBuilder
    implements
        Builder<InterfaceInstanceResponse, InterfaceInstanceResponseBuilder> {
  _$InterfaceInstanceResponse? _$v;

  JsonObject? _config;
  JsonObject? get config => _$this._config;
  set config(JsonObject? config) => _$this._config = config;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _interfaceType;
  String? get interfaceType => _$this._interfaceType;
  set interfaceType(String? interfaceType) =>
      _$this._interfaceType = interfaceType;

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  InterfaceInstanceResponseBuilder() {
    InterfaceInstanceResponse._defaults(this);
  }

  InterfaceInstanceResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _config = $v.config;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _interfaceType = $v.interfaceType;
      _spaceId = $v.spaceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(InterfaceInstanceResponse other) {
    _$v = other as _$InterfaceInstanceResponse;
  }

  @override
  void update(void Function(InterfaceInstanceResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  InterfaceInstanceResponse build() => _build();

  _$InterfaceInstanceResponse _build() {
    final _$result = _$v ??
        _$InterfaceInstanceResponse._(
          config: config,
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'InterfaceInstanceResponse', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'InterfaceInstanceResponse', 'id'),
          interfaceType: BuiltValueNullFieldError.checkNotNull(
              interfaceType, r'InterfaceInstanceResponse', 'interfaceType'),
          spaceId: BuiltValueNullFieldError.checkNotNull(
              spaceId, r'InterfaceInstanceResponse', 'spaceId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
