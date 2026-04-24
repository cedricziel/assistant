// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'space_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SpaceSummary extends SpaceSummary {
  @override
  final String id;
  @override
  final String name;
  @override
  final String slug;

  factory _$SpaceSummary([void Function(SpaceSummaryBuilder)? updates]) =>
      (SpaceSummaryBuilder()..update(updates))._build();

  _$SpaceSummary._({required this.id, required this.name, required this.slug})
      : super._();
  @override
  SpaceSummary rebuild(void Function(SpaceSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SpaceSummaryBuilder toBuilder() => SpaceSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SpaceSummary &&
        id == other.id &&
        name == other.name &&
        slug == other.slug;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SpaceSummary')
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug))
        .toString();
  }
}

class SpaceSummaryBuilder
    implements Builder<SpaceSummary, SpaceSummaryBuilder> {
  _$SpaceSummary? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  SpaceSummaryBuilder() {
    SpaceSummary._defaults(this);
  }

  SpaceSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SpaceSummary other) {
    _$v = other as _$SpaceSummary;
  }

  @override
  void update(void Function(SpaceSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SpaceSummary build() => _build();

  _$SpaceSummary _build() {
    final _$result = _$v ??
        _$SpaceSummary._(
          id: BuiltValueNullFieldError.checkNotNull(id, r'SpaceSummary', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SpaceSummary', 'name'),
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'SpaceSummary', 'slug'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
