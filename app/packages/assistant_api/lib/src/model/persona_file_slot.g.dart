// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_file_slot.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaFileSlot extends PersonaFileSlot {
  @override
  final String description;
  @override
  final String displayName;
  @override
  final bool exists;
  @override
  final String filename;

  factory _$PersonaFileSlot([void Function(PersonaFileSlotBuilder)? updates]) =>
      (PersonaFileSlotBuilder()..update(updates))._build();

  _$PersonaFileSlot._(
      {required this.description,
      required this.displayName,
      required this.exists,
      required this.filename})
      : super._();
  @override
  PersonaFileSlot rebuild(void Function(PersonaFileSlotBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaFileSlotBuilder toBuilder() => PersonaFileSlotBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaFileSlot &&
        description == other.description &&
        displayName == other.displayName &&
        exists == other.exists &&
        filename == other.filename;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, displayName.hashCode);
    _$hash = $jc(_$hash, exists.hashCode);
    _$hash = $jc(_$hash, filename.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaFileSlot')
          ..add('description', description)
          ..add('displayName', displayName)
          ..add('exists', exists)
          ..add('filename', filename))
        .toString();
  }
}

class PersonaFileSlotBuilder
    implements Builder<PersonaFileSlot, PersonaFileSlotBuilder> {
  _$PersonaFileSlot? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _displayName;
  String? get displayName => _$this._displayName;
  set displayName(String? displayName) => _$this._displayName = displayName;

  bool? _exists;
  bool? get exists => _$this._exists;
  set exists(bool? exists) => _$this._exists = exists;

  String? _filename;
  String? get filename => _$this._filename;
  set filename(String? filename) => _$this._filename = filename;

  PersonaFileSlotBuilder() {
    PersonaFileSlot._defaults(this);
  }

  PersonaFileSlotBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _displayName = $v.displayName;
      _exists = $v.exists;
      _filename = $v.filename;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaFileSlot other) {
    _$v = other as _$PersonaFileSlot;
  }

  @override
  void update(void Function(PersonaFileSlotBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaFileSlot build() => _build();

  _$PersonaFileSlot _build() {
    final _$result = _$v ??
        _$PersonaFileSlot._(
          description: BuiltValueNullFieldError.checkNotNull(
              description, r'PersonaFileSlot', 'description'),
          displayName: BuiltValueNullFieldError.checkNotNull(
              displayName, r'PersonaFileSlot', 'displayName'),
          exists: BuiltValueNullFieldError.checkNotNull(
              exists, r'PersonaFileSlot', 'exists'),
          filename: BuiltValueNullFieldError.checkNotNull(
              filename, r'PersonaFileSlot', 'filename'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
