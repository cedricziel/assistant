// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_skill_access_mode_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetSkillAccessModeRequest extends SetSkillAccessModeRequest {
  @override
  final String mode;

  factory _$SetSkillAccessModeRequest(
          [void Function(SetSkillAccessModeRequestBuilder)? updates]) =>
      (SetSkillAccessModeRequestBuilder()..update(updates))._build();

  _$SetSkillAccessModeRequest._({required this.mode}) : super._();
  @override
  SetSkillAccessModeRequest rebuild(
          void Function(SetSkillAccessModeRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetSkillAccessModeRequestBuilder toBuilder() =>
      SetSkillAccessModeRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetSkillAccessModeRequest && mode == other.mode;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, mode.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetSkillAccessModeRequest')
          ..add('mode', mode))
        .toString();
  }
}

class SetSkillAccessModeRequestBuilder
    implements
        Builder<SetSkillAccessModeRequest, SetSkillAccessModeRequestBuilder> {
  _$SetSkillAccessModeRequest? _$v;

  String? _mode;
  String? get mode => _$this._mode;
  set mode(String? mode) => _$this._mode = mode;

  SetSkillAccessModeRequestBuilder() {
    SetSkillAccessModeRequest._defaults(this);
  }

  SetSkillAccessModeRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _mode = $v.mode;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetSkillAccessModeRequest other) {
    _$v = other as _$SetSkillAccessModeRequest;
  }

  @override
  void update(void Function(SetSkillAccessModeRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetSkillAccessModeRequest build() => _build();

  _$SetSkillAccessModeRequest _build() {
    final _$result = _$v ??
        _$SetSkillAccessModeRequest._(
          mode: BuiltValueNullFieldError.checkNotNull(
              mode, r'SetSkillAccessModeRequest', 'mode'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
