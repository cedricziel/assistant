// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_skill_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateSkillRequest extends CreateSkillRequest {
  @override
  final String body;
  @override
  final String description;
  @override
  final String name;

  factory _$CreateSkillRequest(
          [void Function(CreateSkillRequestBuilder)? updates]) =>
      (CreateSkillRequestBuilder()..update(updates))._build();

  _$CreateSkillRequest._(
      {required this.body, required this.description, required this.name})
      : super._();
  @override
  CreateSkillRequest rebuild(
          void Function(CreateSkillRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateSkillRequestBuilder toBuilder() =>
      CreateSkillRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateSkillRequest &&
        body == other.body &&
        description == other.description &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, body.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateSkillRequest')
          ..add('body', body)
          ..add('description', description)
          ..add('name', name))
        .toString();
  }
}

class CreateSkillRequestBuilder
    implements Builder<CreateSkillRequest, CreateSkillRequestBuilder> {
  _$CreateSkillRequest? _$v;

  String? _body;
  String? get body => _$this._body;
  set body(String? body) => _$this._body = body;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreateSkillRequestBuilder() {
    CreateSkillRequest._defaults(this);
  }

  CreateSkillRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _body = $v.body;
      _description = $v.description;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateSkillRequest other) {
    _$v = other as _$CreateSkillRequest;
  }

  @override
  void update(void Function(CreateSkillRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateSkillRequest build() => _build();

  _$CreateSkillRequest _build() {
    final _$result = _$v ??
        _$CreateSkillRequest._(
          body: BuiltValueNullFieldError.checkNotNull(
              body, r'CreateSkillRequest', 'body'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'CreateSkillRequest', 'description'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CreateSkillRequest', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
