// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_from_template_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateFromTemplateRequest extends CreateFromTemplateRequest {
  @override
  final String? name;
  @override
  final String templateId;

  factory _$CreateFromTemplateRequest(
          [void Function(CreateFromTemplateRequestBuilder)? updates]) =>
      (CreateFromTemplateRequestBuilder()..update(updates))._build();

  _$CreateFromTemplateRequest._({this.name, required this.templateId})
      : super._();
  @override
  CreateFromTemplateRequest rebuild(
          void Function(CreateFromTemplateRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateFromTemplateRequestBuilder toBuilder() =>
      CreateFromTemplateRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateFromTemplateRequest &&
        name == other.name &&
        templateId == other.templateId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, templateId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateFromTemplateRequest')
          ..add('name', name)
          ..add('templateId', templateId))
        .toString();
  }
}

class CreateFromTemplateRequestBuilder
    implements
        Builder<CreateFromTemplateRequest, CreateFromTemplateRequestBuilder> {
  _$CreateFromTemplateRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _templateId;
  String? get templateId => _$this._templateId;
  set templateId(String? templateId) => _$this._templateId = templateId;

  CreateFromTemplateRequestBuilder() {
    CreateFromTemplateRequest._defaults(this);
  }

  CreateFromTemplateRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _templateId = $v.templateId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateFromTemplateRequest other) {
    _$v = other as _$CreateFromTemplateRequest;
  }

  @override
  void update(void Function(CreateFromTemplateRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateFromTemplateRequest build() => _build();

  _$CreateFromTemplateRequest _build() {
    final _$result = _$v ??
        _$CreateFromTemplateRequest._(
          name: name,
          templateId: BuiltValueNullFieldError.checkNotNull(
              templateId, r'CreateFromTemplateRequest', 'templateId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
