// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaSummary extends PersonaSummary {
  @override
  final String description;
  @override
  final String id;
  @override
  final bool isDefault;
  @override
  final String name;

  factory _$PersonaSummary([void Function(PersonaSummaryBuilder)? updates]) =>
      (PersonaSummaryBuilder()..update(updates))._build();

  _$PersonaSummary._(
      {required this.description,
      required this.id,
      required this.isDefault,
      required this.name})
      : super._();
  @override
  PersonaSummary rebuild(void Function(PersonaSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaSummaryBuilder toBuilder() => PersonaSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaSummary &&
        description == other.description &&
        id == other.id &&
        isDefault == other.isDefault &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, isDefault.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaSummary')
          ..add('description', description)
          ..add('id', id)
          ..add('isDefault', isDefault)
          ..add('name', name))
        .toString();
  }
}

class PersonaSummaryBuilder
    implements Builder<PersonaSummary, PersonaSummaryBuilder> {
  _$PersonaSummary? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  bool? _isDefault;
  bool? get isDefault => _$this._isDefault;
  set isDefault(bool? isDefault) => _$this._isDefault = isDefault;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  PersonaSummaryBuilder() {
    PersonaSummary._defaults(this);
  }

  PersonaSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _id = $v.id;
      _isDefault = $v.isDefault;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaSummary other) {
    _$v = other as _$PersonaSummary;
  }

  @override
  void update(void Function(PersonaSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaSummary build() => _build();

  _$PersonaSummary _build() {
    final _$result = _$v ??
        _$PersonaSummary._(
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'PersonaSummary', 'description'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'PersonaSummary', 'id'),
          isDefault: BuiltValueNullFieldError.checkNotNull(
              isDefault, r'PersonaSummary', 'isDefault'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'PersonaSummary', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
