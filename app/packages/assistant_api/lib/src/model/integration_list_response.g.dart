// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_list_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$IntegrationListResponse extends IntegrationListResponse {
  @override
  final BuiltList<IntegrationSummary> items;

  factory _$IntegrationListResponse(
          [void Function(IntegrationListResponseBuilder)? updates]) =>
      (IntegrationListResponseBuilder()..update(updates))._build();

  _$IntegrationListResponse._({required this.items}) : super._();
  @override
  IntegrationListResponse rebuild(
          void Function(IntegrationListResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  IntegrationListResponseBuilder toBuilder() =>
      IntegrationListResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is IntegrationListResponse && items == other.items;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, items.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'IntegrationListResponse')
          ..add('items', items))
        .toString();
  }
}

class IntegrationListResponseBuilder
    implements
        Builder<IntegrationListResponse, IntegrationListResponseBuilder> {
  _$IntegrationListResponse? _$v;

  ListBuilder<IntegrationSummary>? _items;
  ListBuilder<IntegrationSummary> get items =>
      _$this._items ??= ListBuilder<IntegrationSummary>();
  set items(ListBuilder<IntegrationSummary>? items) => _$this._items = items;

  IntegrationListResponseBuilder() {
    IntegrationListResponse._defaults(this);
  }

  IntegrationListResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _items = $v.items.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(IntegrationListResponse other) {
    _$v = other as _$IntegrationListResponse;
  }

  @override
  void update(void Function(IntegrationListResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  IntegrationListResponse build() => _build();

  _$IntegrationListResponse _build() {
    _$IntegrationListResponse _$result;
    try {
      _$result = _$v ??
          _$IntegrationListResponse._(
            items: items.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'items';
        items.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'IntegrationListResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
