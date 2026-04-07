// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'add_skill_access_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AddSkillAccessRequest extends AddSkillAccessRequest {
  @override
  final String skillName;

  factory _$AddSkillAccessRequest(
          [void Function(AddSkillAccessRequestBuilder)? updates]) =>
      (AddSkillAccessRequestBuilder()..update(updates))._build();

  _$AddSkillAccessRequest._({required this.skillName}) : super._();
  @override
  AddSkillAccessRequest rebuild(
          void Function(AddSkillAccessRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AddSkillAccessRequestBuilder toBuilder() =>
      AddSkillAccessRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AddSkillAccessRequest && skillName == other.skillName;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, skillName.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AddSkillAccessRequest')
          ..add('skillName', skillName))
        .toString();
  }
}

class AddSkillAccessRequestBuilder
    implements Builder<AddSkillAccessRequest, AddSkillAccessRequestBuilder> {
  _$AddSkillAccessRequest? _$v;

  String? _skillName;
  String? get skillName => _$this._skillName;
  set skillName(String? skillName) => _$this._skillName = skillName;

  AddSkillAccessRequestBuilder() {
    AddSkillAccessRequest._defaults(this);
  }

  AddSkillAccessRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _skillName = $v.skillName;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AddSkillAccessRequest other) {
    _$v = other as _$AddSkillAccessRequest;
  }

  @override
  void update(void Function(AddSkillAccessRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AddSkillAccessRequest build() => _build();

  _$AddSkillAccessRequest _build() {
    final _$result = _$v ??
        _$AddSkillAccessRequest._(
          skillName: BuiltValueNullFieldError.checkNotNull(
              skillName, r'AddSkillAccessRequest', 'skillName'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
