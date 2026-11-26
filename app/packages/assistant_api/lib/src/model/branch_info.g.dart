// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'branch_info.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$BranchInfo extends BranchInfo {
  @override
  final int count;
  @override
  final int index;
  @override
  final BuiltList<String> siblingIds;

  factory _$BranchInfo([void Function(BranchInfoBuilder)? updates]) =>
      (BranchInfoBuilder()..update(updates))._build();

  _$BranchInfo._(
      {required this.count, required this.index, required this.siblingIds})
      : super._();
  @override
  BranchInfo rebuild(void Function(BranchInfoBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  BranchInfoBuilder toBuilder() => BranchInfoBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is BranchInfo &&
        count == other.count &&
        index == other.index &&
        siblingIds == other.siblingIds;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, count.hashCode);
    _$hash = $jc(_$hash, index.hashCode);
    _$hash = $jc(_$hash, siblingIds.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'BranchInfo')
          ..add('count', count)
          ..add('index', index)
          ..add('siblingIds', siblingIds))
        .toString();
  }
}

class BranchInfoBuilder implements Builder<BranchInfo, BranchInfoBuilder> {
  _$BranchInfo? _$v;

  int? _count;
  int? get count => _$this._count;
  set count(int? count) => _$this._count = count;

  int? _index;
  int? get index => _$this._index;
  set index(int? index) => _$this._index = index;

  ListBuilder<String>? _siblingIds;
  ListBuilder<String> get siblingIds =>
      _$this._siblingIds ??= ListBuilder<String>();
  set siblingIds(ListBuilder<String>? siblingIds) =>
      _$this._siblingIds = siblingIds;

  BranchInfoBuilder() {
    BranchInfo._defaults(this);
  }

  BranchInfoBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _count = $v.count;
      _index = $v.index;
      _siblingIds = $v.siblingIds.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(BranchInfo other) {
    _$v = other as _$BranchInfo;
  }

  @override
  void update(void Function(BranchInfoBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  BranchInfo build() => _build();

  _$BranchInfo _build() {
    _$BranchInfo _$result;
    try {
      _$result = _$v ??
          _$BranchInfo._(
            count: BuiltValueNullFieldError.checkNotNull(
                count, r'BranchInfo', 'count'),
            index: BuiltValueNullFieldError.checkNotNull(
                index, r'BranchInfo', 'index'),
            siblingIds: siblingIds.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'siblingIds';
        siblingIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'BranchInfo', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
