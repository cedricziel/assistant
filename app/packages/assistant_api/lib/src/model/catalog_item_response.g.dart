// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_item_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CatalogItemResponse extends CatalogItemResponse {
  @override
  final String createdAt;
  @override
  final String description;
  @override
  final String id;
  @override
  final String name;
  @override
  final String resourceType;

  factory _$CatalogItemResponse(
          [void Function(CatalogItemResponseBuilder)? updates]) =>
      (CatalogItemResponseBuilder()..update(updates))._build();

  _$CatalogItemResponse._(
      {required this.createdAt,
      required this.description,
      required this.id,
      required this.name,
      required this.resourceType})
      : super._();
  @override
  CatalogItemResponse rebuild(
          void Function(CatalogItemResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CatalogItemResponseBuilder toBuilder() =>
      CatalogItemResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CatalogItemResponse &&
        createdAt == other.createdAt &&
        description == other.description &&
        id == other.id &&
        name == other.name &&
        resourceType == other.resourceType;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, resourceType.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CatalogItemResponse')
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('id', id)
          ..add('name', name)
          ..add('resourceType', resourceType))
        .toString();
  }
}

class CatalogItemResponseBuilder
    implements Builder<CatalogItemResponse, CatalogItemResponseBuilder> {
  _$CatalogItemResponse? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _resourceType;
  String? get resourceType => _$this._resourceType;
  set resourceType(String? resourceType) => _$this._resourceType = resourceType;

  CatalogItemResponseBuilder() {
    CatalogItemResponse._defaults(this);
  }

  CatalogItemResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _description = $v.description;
      _id = $v.id;
      _name = $v.name;
      _resourceType = $v.resourceType;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CatalogItemResponse other) {
    _$v = other as _$CatalogItemResponse;
  }

  @override
  void update(void Function(CatalogItemResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CatalogItemResponse build() => _build();

  _$CatalogItemResponse _build() {
    final _$result = _$v ??
        _$CatalogItemResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'CatalogItemResponse', 'createdAt'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'CatalogItemResponse', 'description'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'CatalogItemResponse', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CatalogItemResponse', 'name'),
          resourceType: BuiltValueNullFieldError.checkNotNull(
              resourceType, r'CatalogItemResponse', 'resourceType'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
