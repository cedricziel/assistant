// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'string_list.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$StringList extends StringList {
  @override
  final BuiltList<String>? list;

  factory _$StringList([void Function(StringListBuilder)? updates]) =>
      (StringListBuilder()..update(updates))._build();

  _$StringList._({this.list}) : super._();
  @override
  StringList rebuild(void Function(StringListBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  StringListBuilder toBuilder() => StringListBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is StringList && list == other.list;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, list.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'StringList')..add('list', list))
        .toString();
  }
}

class StringListBuilder implements Builder<StringList, StringListBuilder> {
  _$StringList? _$v;

  ListBuilder<String>? _list;
  ListBuilder<String> get list => _$this._list ??= ListBuilder<String>();
  set list(ListBuilder<String>? list) => _$this._list = list;

  StringListBuilder() {
    StringList._defaults(this);
  }

  StringListBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _list = $v.list?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(StringList other) {
    _$v = other as _$StringList;
  }

  @override
  void update(void Function(StringListBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  StringList build() => _build();

  _$StringList _build() {
    _$StringList _$result;
    try {
      _$result = _$v ??
          _$StringList._(
            list: _list?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'list';
        _list?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'StringList', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
