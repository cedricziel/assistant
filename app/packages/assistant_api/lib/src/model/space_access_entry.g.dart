// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'space_access_entry.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SpaceAccessEntry extends SpaceAccessEntry {
  @override
  final SpaceAccessMode mode;
  @override
  final String spaceId;

  factory _$SpaceAccessEntry(
          [void Function(SpaceAccessEntryBuilder)? updates]) =>
      (SpaceAccessEntryBuilder()..update(updates))._build();

  _$SpaceAccessEntry._({required this.mode, required this.spaceId}) : super._();
  @override
  SpaceAccessEntry rebuild(void Function(SpaceAccessEntryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SpaceAccessEntryBuilder toBuilder() =>
      SpaceAccessEntryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SpaceAccessEntry &&
        mode == other.mode &&
        spaceId == other.spaceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, mode.hashCode);
    _$hash = $jc(_$hash, spaceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SpaceAccessEntry')
          ..add('mode', mode)
          ..add('spaceId', spaceId))
        .toString();
  }
}

class SpaceAccessEntryBuilder
    implements Builder<SpaceAccessEntry, SpaceAccessEntryBuilder> {
  _$SpaceAccessEntry? _$v;

  SpaceAccessMode? _mode;
  SpaceAccessMode? get mode => _$this._mode;
  set mode(SpaceAccessMode? mode) => _$this._mode = mode;

  String? _spaceId;
  String? get spaceId => _$this._spaceId;
  set spaceId(String? spaceId) => _$this._spaceId = spaceId;

  SpaceAccessEntryBuilder() {
    SpaceAccessEntry._defaults(this);
  }

  SpaceAccessEntryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _mode = $v.mode;
      _spaceId = $v.spaceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SpaceAccessEntry other) {
    _$v = other as _$SpaceAccessEntry;
  }

  @override
  void update(void Function(SpaceAccessEntryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SpaceAccessEntry build() => _build();

  _$SpaceAccessEntry _build() {
    final _$result = _$v ??
        _$SpaceAccessEntry._(
          mode: BuiltValueNullFieldError.checkNotNull(
              mode, r'SpaceAccessEntry', 'mode'),
          spaceId: BuiltValueNullFieldError.checkNotNull(
              spaceId, r'SpaceAccessEntry', 'spaceId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
