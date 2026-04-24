// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'org_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OrgSummary extends OrgSummary {
  @override
  final String authMode;
  @override
  final String id;
  @override
  final String name;
  @override
  final String slug;

  factory _$OrgSummary([void Function(OrgSummaryBuilder)? updates]) =>
      (OrgSummaryBuilder()..update(updates))._build();

  _$OrgSummary._(
      {required this.authMode,
      required this.id,
      required this.name,
      required this.slug})
      : super._();
  @override
  OrgSummary rebuild(void Function(OrgSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OrgSummaryBuilder toBuilder() => OrgSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OrgSummary &&
        authMode == other.authMode &&
        id == other.id &&
        name == other.name &&
        slug == other.slug;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authMode.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OrgSummary')
          ..add('authMode', authMode)
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug))
        .toString();
  }
}

class OrgSummaryBuilder implements Builder<OrgSummary, OrgSummaryBuilder> {
  _$OrgSummary? _$v;

  String? _authMode;
  String? get authMode => _$this._authMode;
  set authMode(String? authMode) => _$this._authMode = authMode;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  OrgSummaryBuilder() {
    OrgSummary._defaults(this);
  }

  OrgSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authMode = $v.authMode;
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OrgSummary other) {
    _$v = other as _$OrgSummary;
  }

  @override
  void update(void Function(OrgSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OrgSummary build() => _build();

  _$OrgSummary _build() {
    final _$result = _$v ??
        _$OrgSummary._(
          authMode: BuiltValueNullFieldError.checkNotNull(
              authMode, r'OrgSummary', 'authMode'),
          id: BuiltValueNullFieldError.checkNotNull(id, r'OrgSummary', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'OrgSummary', 'name'),
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'OrgSummary', 'slug'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
