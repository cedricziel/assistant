// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_entry_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CatalogEntryListResponse extends CatalogEntryListResponse {
  @override
  final BuiltList<CatalogEntrySummary> items;
  @override
  final String? nextCursor;

  factory _$CatalogEntryListResponse(
          [void Function(CatalogEntryListResponseBuilder)? updates]) =>
      (CatalogEntryListResponseBuilder()..update(updates))._build();

  _$CatalogEntryListResponse._({required this.items, this.nextCursor})
      : super._();
  @override
  CatalogEntryListResponse rebuild(
          void Function(CatalogEntryListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CatalogEntryListResponseBuilder toBuilder() =>
      CatalogEntryListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CatalogEntryListResponse &&
        items == other.items &&
        nextCursor == other.nextCursor;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jc(_$hash, nextCursor.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CatalogEntryListResponse')
          ..add('items', items)
          ..add('nextCursor', nextCursor))
        .toString();
  }
}

class CatalogEntryListResponseBuilder
    implements
        Builder<CatalogEntryListResponse, CatalogEntryListResponseBuilder> {
  _$CatalogEntryListResponse? _$v;

  ListBuilder<CatalogEntrySummary>? _items;
  ListBuilder<CatalogEntrySummary> get items =>
      _$this._items ??= ListBuilder<CatalogEntrySummary>();
  set items(ListBuilder<CatalogEntrySummary>? items) => _$this._items = items;

  String? _nextCursor;
  String? get nextCursor => _$this._nextCursor;
  set nextCursor(String? nextCursor) => _$this._nextCursor = nextCursor;

  CatalogEntryListResponseBuilder() {
    CatalogEntryListResponse._defaults(this);
  }

  CatalogEntryListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _nextCursor = $v.nextCursor;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CatalogEntryListResponse other) {
    _$v = other as _$CatalogEntryListResponse;
  }

  @override
  void update(void Function(CatalogEntryListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CatalogEntryListResponse build() => _build();

  _$CatalogEntryListResponse _build() {
    _$CatalogEntryListResponse _$result;
    try {
      _$result = _$v ??
          _$CatalogEntryListResponse._(
            items: items.build(),
            nextCursor: nextCursor,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CatalogEntryListResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
