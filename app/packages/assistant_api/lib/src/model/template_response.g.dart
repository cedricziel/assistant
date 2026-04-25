// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'template_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TemplateResponse extends TemplateResponse {
  @override
  final String createdAt;
  @override
  final String description;
  @override
  final String id;
  @override
  final String name;

  factory _$TemplateResponse(
          [void Function(TemplateResponseBuilder)? updates]) =>
      (TemplateResponseBuilder()..update(updates))._build();

  _$TemplateResponse._(
      {required this.createdAt,
      required this.description,
      required this.id,
      required this.name})
      : super._();
  @override
  TemplateResponse rebuild(void Function(TemplateResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TemplateResponseBuilder toBuilder() =>
      TemplateResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TemplateResponse &&
        createdAt == other.createdAt &&
        description == other.description &&
        id == other.id &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TemplateResponse')
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('id', id)
          ..add('name', name))
        .toString();
  }
}

class TemplateResponseBuilder
    implements Builder<TemplateResponse, TemplateResponseBuilder> {
  _$TemplateResponse? _$v;

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

  TemplateResponseBuilder() {
    TemplateResponse._defaults(this);
  }

  TemplateResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _description = $v.description;
      _id = $v.id;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TemplateResponse other) {
    _$v = other as _$TemplateResponse;
  }

  @override
  void update(void Function(TemplateResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TemplateResponse build() => _build();

  _$TemplateResponse _build() {
    final _$result = _$v ??
        _$TemplateResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'TemplateResponse', 'createdAt'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'TemplateResponse', 'description'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'TemplateResponse', 'id'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'TemplateResponse', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
