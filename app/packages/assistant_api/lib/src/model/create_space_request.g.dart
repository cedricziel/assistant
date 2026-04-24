// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_space_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateSpaceRequest extends CreateSpaceRequest {
  @override
  final String name;
  @override
  final String slug;

  factory _$CreateSpaceRequest(
          [void Function(CreateSpaceRequestBuilder)? updates]) =>
      (CreateSpaceRequestBuilder()..update(updates))._build();

  _$CreateSpaceRequest._({required this.name, required this.slug}) : super._();
  @override
  CreateSpaceRequest rebuild(
          void Function(CreateSpaceRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateSpaceRequestBuilder toBuilder() =>
      CreateSpaceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateSpaceRequest &&
        name == other.name &&
        slug == other.slug;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateSpaceRequest')
          ..add('name', name)
          ..add('slug', slug))
        .toString();
  }
}

class CreateSpaceRequestBuilder
    implements Builder<CreateSpaceRequest, CreateSpaceRequestBuilder> {
  _$CreateSpaceRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  CreateSpaceRequestBuilder() {
    CreateSpaceRequest._defaults(this);
  }

  CreateSpaceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _slug = $v.slug;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateSpaceRequest other) {
    _$v = other as _$CreateSpaceRequest;
  }

  @override
  void update(void Function(CreateSpaceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateSpaceRequest build() => _build();

  _$CreateSpaceRequest _build() {
    final _$result = _$v ??
        _$CreateSpaceRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CreateSpaceRequest', 'name'),
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'CreateSpaceRequest', 'slug'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
