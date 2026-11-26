// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'model_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ModelSummary extends ModelSummary {
  @override
  final bool isDefault;
  @override
  final String name;

  factory _$ModelSummary([void Function(ModelSummaryBuilder)? updates]) =>
      (ModelSummaryBuilder()..update(updates))._build();

  _$ModelSummary._({required this.isDefault, required this.name}) : super._();
  @override
  ModelSummary rebuild(void Function(ModelSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ModelSummaryBuilder toBuilder() => ModelSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ModelSummary &&
        isDefault == other.isDefault &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, isDefault.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ModelSummary')
          ..add('isDefault', isDefault)
          ..add('name', name))
        .toString();
  }
}

class ModelSummaryBuilder
    implements Builder<ModelSummary, ModelSummaryBuilder> {
  _$ModelSummary? _$v;

  bool? _isDefault;
  bool? get isDefault => _$this._isDefault;
  set isDefault(bool? isDefault) => _$this._isDefault = isDefault;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  ModelSummaryBuilder() {
    ModelSummary._defaults(this);
  }

  ModelSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _isDefault = $v.isDefault;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ModelSummary other) {
    _$v = other as _$ModelSummary;
  }

  @override
  void update(void Function(ModelSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ModelSummary build() => _build();

  _$ModelSummary _build() {
    final _$result = _$v ??
        _$ModelSummary._(
          isDefault: BuiltValueNullFieldError.checkNotNull(
              isDefault, r'ModelSummary', 'isDefault'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'ModelSummary', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
