// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_skill_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateSkillRequest extends UpdateSkillRequest {
  @override
  final String body;
  @override
  final String description;

  factory _$UpdateSkillRequest(
          [void Function(UpdateSkillRequestBuilder)? updates]) =>
      (UpdateSkillRequestBuilder()..update(updates))._build();

  _$UpdateSkillRequest._({required this.body, required this.description})
      : super._();
  @override
  UpdateSkillRequest rebuild(
          void Function(UpdateSkillRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateSkillRequestBuilder toBuilder() =>
      UpdateSkillRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateSkillRequest &&
        body == other.body &&
        description == other.description;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, body.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateSkillRequest')
          ..add('body', body)
          ..add('description', description))
        .toString();
  }
}

class UpdateSkillRequestBuilder
    implements Builder<UpdateSkillRequest, UpdateSkillRequestBuilder> {
  _$UpdateSkillRequest? _$v;

  String? _body;
  String? get body => _$this._body;
  set body(String? body) => _$this._body = body;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  UpdateSkillRequestBuilder() {
    UpdateSkillRequest._defaults(this);
  }

  UpdateSkillRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _body = $v.body;
      _description = $v.description;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateSkillRequest other) {
    _$v = other as _$UpdateSkillRequest;
  }

  @override
  void update(void Function(UpdateSkillRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateSkillRequest build() => _build();

  _$UpdateSkillRequest _build() {
    final _$result = _$v ??
        _$UpdateSkillRequest._(
          body: BuiltValueNullFieldError.checkNotNull(
              body, r'UpdateSkillRequest', 'body'),
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'UpdateSkillRequest', 'description'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
