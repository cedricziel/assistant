// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'search_segment.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SearchSegment extends SearchSegment {
  @override
  final bool highlighted;
  @override
  final String text;

  factory _$SearchSegment([void Function(SearchSegmentBuilder)? updates]) =>
      (SearchSegmentBuilder()..update(updates))._build();

  _$SearchSegment._({required this.highlighted, required this.text})
      : super._();
  @override
  SearchSegment rebuild(void Function(SearchSegmentBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SearchSegmentBuilder toBuilder() => SearchSegmentBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SearchSegment &&
        highlighted == other.highlighted &&
        text == other.text;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, highlighted.hashCode);
    _$hash = $jc(_$hash, text.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SearchSegment')
          ..add('highlighted', highlighted)
          ..add('text', text))
        .toString();
  }
}

class SearchSegmentBuilder
    implements Builder<SearchSegment, SearchSegmentBuilder> {
  _$SearchSegment? _$v;

  bool? _highlighted;
  bool? get highlighted => _$this._highlighted;
  set highlighted(bool? highlighted) => _$this._highlighted = highlighted;

  String? _text;
  String? get text => _$this._text;
  set text(String? text) => _$this._text = text;

  SearchSegmentBuilder() {
    SearchSegment._defaults(this);
  }

  SearchSegmentBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _highlighted = $v.highlighted;
      _text = $v.text;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SearchSegment other) {
    _$v = other as _$SearchSegment;
  }

  @override
  void update(void Function(SearchSegmentBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SearchSegment build() => _build();

  _$SearchSegment _build() {
    final _$result = _$v ??
        _$SearchSegment._(
          highlighted: BuiltValueNullFieldError.checkNotNull(
              highlighted, r'SearchSegment', 'highlighted'),
          text: BuiltValueNullFieldError.checkNotNull(
              text, r'SearchSegment', 'text'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
