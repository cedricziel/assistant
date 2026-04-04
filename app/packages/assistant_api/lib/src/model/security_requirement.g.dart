// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'security_requirement.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SecurityRequirement extends SecurityRequirement {
  @override
  final BuiltMap<String, StringList>? schemes;

  factory _$SecurityRequirement(
          [void Function(SecurityRequirementBuilder)? updates]) =>
      (SecurityRequirementBuilder()..update(updates))._build();

  _$SecurityRequirement._({this.schemes}) : super._();
  @override
  SecurityRequirement rebuild(
          void Function(SecurityRequirementBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SecurityRequirementBuilder toBuilder() =>
      SecurityRequirementBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SecurityRequirement && schemes == other.schemes;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, schemes.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SecurityRequirement')
          ..add('schemes', schemes))
        .toString();
  }
}

class SecurityRequirementBuilder
    implements Builder<SecurityRequirement, SecurityRequirementBuilder> {
  _$SecurityRequirement? _$v;

  MapBuilder<String, StringList>? _schemes;
  MapBuilder<String, StringList> get schemes =>
      _$this._schemes ??= MapBuilder<String, StringList>();
  set schemes(MapBuilder<String, StringList>? schemes) =>
      _$this._schemes = schemes;

  SecurityRequirementBuilder() {
    SecurityRequirement._defaults(this);
  }

  SecurityRequirementBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _schemes = $v.schemes?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SecurityRequirement other) {
    _$v = other as _$SecurityRequirement;
  }

  @override
  void update(void Function(SecurityRequirementBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SecurityRequirement build() => _build();

  _$SecurityRequirement _build() {
    _$SecurityRequirement _$result;
    try {
      _$result = _$v ??
          _$SecurityRequirement._(
            schemes: _schemes?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'schemes';
        _schemes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SecurityRequirement', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
