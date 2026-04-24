// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'space_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SpaceDetail extends SpaceDetail {
  @override
  final String createdAt;
  @override
  final String id;
  @override
  final String name;
  @override
  final String orgId;
  @override
  final String slug;
  @override
  final String updatedAt;

  factory _$SpaceDetail([void Function(SpaceDetailBuilder)? updates]) =>
      (SpaceDetailBuilder()..update(updates))._build();

  _$SpaceDetail._(
      {required this.createdAt,
      required this.id,
      required this.name,
      required this.orgId,
      required this.slug,
      required this.updatedAt})
      : super._();
  @override
  SpaceDetail rebuild(void Function(SpaceDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SpaceDetailBuilder toBuilder() => SpaceDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SpaceDetail &&
        createdAt == other.createdAt &&
        id == other.id &&
        name == other.name &&
        orgId == other.orgId &&
        slug == other.slug &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, orgId.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SpaceDetail')
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('name', name)
          ..add('orgId', orgId)
          ..add('slug', slug)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class SpaceDetailBuilder implements Builder<SpaceDetail, SpaceDetailBuilder> {
  _$SpaceDetail? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _orgId;
  String? get orgId => _$this._orgId;
  set orgId(String? orgId) => _$this._orgId = orgId;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  SpaceDetailBuilder() {
    SpaceDetail._defaults(this);
  }

  SpaceDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _id = $v.id;
      _name = $v.name;
      _orgId = $v.orgId;
      _slug = $v.slug;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SpaceDetail other) {
    _$v = other as _$SpaceDetail;
  }

  @override
  void update(void Function(SpaceDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SpaceDetail build() => _build();

  _$SpaceDetail _build() {
    final _$result = _$v ??
        _$SpaceDetail._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'SpaceDetail', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'SpaceDetail', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SpaceDetail', 'name'),
          orgId: BuiltValueNullFieldError.checkNotNull(
              orgId, r'SpaceDetail', 'orgId'),
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'SpaceDetail', 'slug'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'SpaceDetail', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
