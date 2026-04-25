// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'publish_catalog_item_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PublishCatalogItemRequest extends PublishCatalogItemRequest {
  @override
  final String? description;
  @override
  final String name;
  @override
  final String resourceType;

  factory _$PublishCatalogItemRequest(
          [void Function(PublishCatalogItemRequestBuilder)? updates]) =>
      (PublishCatalogItemRequestBuilder()..update(updates))._build();

  _$PublishCatalogItemRequest._(
      {this.description, required this.name, required this.resourceType})
      : super._();
  @override
  PublishCatalogItemRequest rebuild(
          void Function(PublishCatalogItemRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PublishCatalogItemRequestBuilder toBuilder() =>
      PublishCatalogItemRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PublishCatalogItemRequest &&
        description == other.description &&
        name == other.name &&
        resourceType == other.resourceType;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, resourceType.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PublishCatalogItemRequest')
          ..add('description', description)
          ..add('name', name)
          ..add('resourceType', resourceType))
        .toString();
  }
}

class PublishCatalogItemRequestBuilder
    implements
        Builder<PublishCatalogItemRequest, PublishCatalogItemRequestBuilder> {
  _$PublishCatalogItemRequest? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _resourceType;
  String? get resourceType => _$this._resourceType;
  set resourceType(String? resourceType) => _$this._resourceType = resourceType;

  PublishCatalogItemRequestBuilder() {
    PublishCatalogItemRequest._defaults(this);
  }

  PublishCatalogItemRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _name = $v.name;
      _resourceType = $v.resourceType;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PublishCatalogItemRequest other) {
    _$v = other as _$PublishCatalogItemRequest;
  }

  @override
  void update(void Function(PublishCatalogItemRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PublishCatalogItemRequest build() => _build();

  _$PublishCatalogItemRequest _build() {
    final _$result = _$v ??
        _$PublishCatalogItemRequest._(
          description: description,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'PublishCatalogItemRequest', 'name'),
          resourceType: BuiltValueNullFieldError.checkNotNull(
              resourceType, r'PublishCatalogItemRequest', 'resourceType'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
