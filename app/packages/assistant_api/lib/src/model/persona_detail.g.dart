// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaDetail extends PersonaDetail {
  @override
  final bool canEditStarterPrompts;
  @override
  final String createdAt;
  @override
  final BuiltList<PersonaFileSlot> files;
  @override
  final String id;
  @override
  final String name;
  @override
  final String skillAccessMode;
  @override
  final BuiltList<String> starterPrompts;
  @override
  final int? turnTimeoutSecs;
  @override
  final String updatedAt;

  factory _$PersonaDetail([void Function(PersonaDetailBuilder)? updates]) =>
      (PersonaDetailBuilder()..update(updates))._build();

  _$PersonaDetail._(
      {required this.canEditStarterPrompts,
      required this.createdAt,
      required this.files,
      required this.id,
      required this.name,
      required this.skillAccessMode,
      required this.starterPrompts,
      this.turnTimeoutSecs,
      required this.updatedAt})
      : super._();
  @override
  PersonaDetail rebuild(void Function(PersonaDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaDetailBuilder toBuilder() => PersonaDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaDetail &&
        canEditStarterPrompts == other.canEditStarterPrompts &&
        createdAt == other.createdAt &&
        files == other.files &&
        id == other.id &&
        name == other.name &&
        skillAccessMode == other.skillAccessMode &&
        starterPrompts == other.starterPrompts &&
        turnTimeoutSecs == other.turnTimeoutSecs &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, canEditStarterPrompts.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, files.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, skillAccessMode.hashCode);
    _$hash = $jc(_$hash, starterPrompts.hashCode);
    _$hash = $jc(_$hash, turnTimeoutSecs.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaDetail')
          ..add('canEditStarterPrompts', canEditStarterPrompts)
          ..add('createdAt', createdAt)
          ..add('files', files)
          ..add('id', id)
          ..add('name', name)
          ..add('skillAccessMode', skillAccessMode)
          ..add('starterPrompts', starterPrompts)
          ..add('turnTimeoutSecs', turnTimeoutSecs)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class PersonaDetailBuilder
    implements Builder<PersonaDetail, PersonaDetailBuilder> {
  _$PersonaDetail? _$v;

  bool? _canEditStarterPrompts;
  bool? get canEditStarterPrompts => _$this._canEditStarterPrompts;
  set canEditStarterPrompts(bool? canEditStarterPrompts) =>
      _$this._canEditStarterPrompts = canEditStarterPrompts;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  ListBuilder<PersonaFileSlot>? _files;
  ListBuilder<PersonaFileSlot> get files =>
      _$this._files ??= ListBuilder<PersonaFileSlot>();
  set files(ListBuilder<PersonaFileSlot>? files) => _$this._files = files;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _skillAccessMode;
  String? get skillAccessMode => _$this._skillAccessMode;
  set skillAccessMode(String? skillAccessMode) =>
      _$this._skillAccessMode = skillAccessMode;

  ListBuilder<String>? _starterPrompts;
  ListBuilder<String> get starterPrompts =>
      _$this._starterPrompts ??= ListBuilder<String>();
  set starterPrompts(ListBuilder<String>? starterPrompts) =>
      _$this._starterPrompts = starterPrompts;

  int? _turnTimeoutSecs;
  int? get turnTimeoutSecs => _$this._turnTimeoutSecs;
  set turnTimeoutSecs(int? turnTimeoutSecs) =>
      _$this._turnTimeoutSecs = turnTimeoutSecs;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  PersonaDetailBuilder() {
    PersonaDetail._defaults(this);
  }

  PersonaDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _canEditStarterPrompts = $v.canEditStarterPrompts;
      _createdAt = $v.createdAt;
      _files = $v.files.toBuilder();
      _id = $v.id;
      _name = $v.name;
      _skillAccessMode = $v.skillAccessMode;
      _starterPrompts = $v.starterPrompts.toBuilder();
      _turnTimeoutSecs = $v.turnTimeoutSecs;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaDetail other) {
    _$v = other as _$PersonaDetail;
  }

  @override
  void update(void Function(PersonaDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaDetail build() => _build();

  _$PersonaDetail _build() {
    _$PersonaDetail _$result;
    try {
      _$result = _$v ??
          _$PersonaDetail._(
            canEditStarterPrompts: BuiltValueNullFieldError.checkNotNull(
                canEditStarterPrompts,
                r'PersonaDetail',
                'canEditStarterPrompts'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'PersonaDetail', 'createdAt'),
            files: files.build(),
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'PersonaDetail', 'id'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'PersonaDetail', 'name'),
            skillAccessMode: BuiltValueNullFieldError.checkNotNull(
                skillAccessMode, r'PersonaDetail', 'skillAccessMode'),
            starterPrompts: starterPrompts.build(),
            turnTimeoutSecs: turnTimeoutSecs,
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'PersonaDetail', 'updatedAt'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'files';
        files.build();

        _$failedField = 'starterPrompts';
        starterPrompts.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'PersonaDetail', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
