// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_org_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateOrgRequest extends CreateOrgRequest {
  @override
  final String? authMode;
  @override
  final String name;
  @override
  final String slug;

  factory _$CreateOrgRequest(
          [void Function(CreateOrgRequestBuilder)? updates]) =>
      (CreateOrgRequestBuilder()..update(updates))._build();

  _$CreateOrgRequest._({this.authMode, required this.name, required this.slug})
      : super._();
  @override
  CreateOrgRequest rebuild(void Function(CreateOrgRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateOrgRequestBuilder toBuilder() =>
      CreateOrgRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateOrgRequest &&
        authMode == other.authMode &&
        name == other.name &&
        slug == other.slug;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authMode.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateOrgRequest')
          ..add('authMode', authMode)
          ..add('name', name)
          ..add('slug', slug))
        .toString();
  }
}

class CreateOrgRequestBuilder
    implements Builder<CreateOrgRequest, CreateOrgRequestBuilder> {
  _$CreateOrgRequest? _$v;

  String? _authMode;
  String? get authMode => _$this._authMode;
  set authMode(String? authMode) => _$this._authMode = authMode;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  CreateOrgRequestBuilder() {
    CreateOrgRequest._defaults(this);
  }

  CreateOrgRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authMode = $v.authMode;
      _name = $v.name;
      _slug = $v.slug;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateOrgRequest other) {
    _$v = other as _$CreateOrgRequest;
  }

  @override
  void update(void Function(CreateOrgRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateOrgRequest build() => _build();

  _$CreateOrgRequest _build() {
    final _$result = _$v ??
        _$CreateOrgRequest._(
          authMode: authMode,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CreateOrgRequest', 'name'),
          slug: BuiltValueNullFieldError.checkNotNull(
              slug, r'CreateOrgRequest', 'slug'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
