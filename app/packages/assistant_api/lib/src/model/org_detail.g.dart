// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'org_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OrgDetail extends OrgDetail {
  @override
  final String authMode;
  @override
  final String createdAt;
  @override
  final String id;
  @override
  final String name;
  @override
  final String slug;
  @override
  final String updatedAt;

  factory _$OrgDetail([void Function(OrgDetailBuilder)? updates]) =>
      (OrgDetailBuilder()..update(updates))._build();

  _$OrgDetail._(
      {required this.authMode,
      required this.createdAt,
      required this.id,
      required this.name,
      required this.slug,
      required this.updatedAt})
      : super._();
  @override
  OrgDetail rebuild(void Function(OrgDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OrgDetailBuilder toBuilder() => OrgDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OrgDetail &&
        authMode == other.authMode &&
        createdAt == other.createdAt &&
        id == other.id &&
        name == other.name &&
        slug == other.slug &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authMode.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OrgDetail')
          ..add('authMode', authMode)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class OrgDetailBuilder implements Builder<OrgDetail, OrgDetailBuilder> {
  _$OrgDetail? _$v;

  String? _authMode;
  String? get authMode => _$this._authMode;
  set authMode(String? authMode) => _$this._authMode = authMode;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  OrgDetailBuilder() {
    OrgDetail._defaults(this);
  }

  OrgDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authMode = $v.authMode;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OrgDetail other) {
    _$v = other as _$OrgDetail;
  }

  @override
  void update(void Function(OrgDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OrgDetail build() => _build();

  _$OrgDetail _build() {
    final _$result = _$v ??
        _$OrgDetail._(
          authMode: BuiltValueNullFieldError.checkNotNull(
              authMode, r'OrgDetail', 'authMode'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'OrgDetail', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'OrgDetail', 'id'),
          name:
              BuiltValueNullFieldError.checkNotNull(name, r'OrgDetail', 'name'),
          slug:
              BuiltValueNullFieldError.checkNotNull(slug, r'OrgDetail', 'slug'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'OrgDetail', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
