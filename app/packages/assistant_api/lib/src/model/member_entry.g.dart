// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'member_entry.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MemberEntry extends MemberEntry {
  @override
  final String createdAt;
  @override
  final String role;
  @override
  final String spaceId;
  @override
  final String userId;

  factory _$MemberEntry([void Function(MemberEntryBuilder)? updates]) =>
      (MemberEntryBuilder()..update(updates))._build();

  _$MemberEntry._(
      {required this.createdAt,
      required this.role,
      required this.spaceId,
      required this.userId})
      : super._();
  @override
  MemberEntry rebuild(void Function(MemberEntryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MemberEntryBuilder toBuilder() => MemberEntryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MemberEntry &&
        createdAt == other.createdAt &&
        role == other.role &&
        spaceId == other.spaceId &&
        userId == other.userId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jc(_$hash, userId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MemberEntry')
          ..add('createdAt', createdAt)
          ..add('role', role)
          ..add('spaceId', spaceId)
          ..add('userId', userId))
        .toString();
  }
}

class MemberEntryBuilder implements Builder<MemberEntry, MemberEntryBuilder> {
  _$MemberEntry? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  String? _userId;
  String? get userId => _$this._userId;
  set userId(String? userId) => _$this._userId = userId;

  MemberEntryBuilder() {
    MemberEntry._defaults(this);
  }

  MemberEntryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _role = $v.role;
      _spaceId = $v.spaceId;
      _userId = $v.userId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MemberEntry other) {
    _$v = other as _$MemberEntry;
  }

  @override
  void update(void Function(MemberEntryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MemberEntry build() => _build();

  _$MemberEntry _build() {
    final _$result = _$v ??
        _$MemberEntry._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'MemberEntry', 'createdAt'),
          role: BuiltValueNullFieldError.checkNotNull(
              role, r'MemberEntry', 'role'),
          spaceId: BuiltValueNullFieldError.checkNotNull(
              spaceId, r'MemberEntry', 'spaceId'),
          userId: BuiltValueNullFieldError.checkNotNull(
              userId, r'MemberEntry', 'userId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
