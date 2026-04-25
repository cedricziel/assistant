// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'subscription_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SubscriptionResponse extends SubscriptionResponse {
  @override
  final String catalogItemId;
  @override
  final String createdAt;
  @override
  final String id;

  factory _$SubscriptionResponse(
          [void Function(SubscriptionResponseBuilder)? updates]) =>
      (SubscriptionResponseBuilder()..update(updates))._build();

  _$SubscriptionResponse._(
      {required this.catalogItemId, required this.createdAt, required this.id})
      : super._();
  @override
  SubscriptionResponse rebuild(
          void Function(SubscriptionResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SubscriptionResponseBuilder toBuilder() =>
      SubscriptionResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SubscriptionResponse &&
        catalogItemId == other.catalogItemId &&
        createdAt == other.createdAt &&
        id == other.id;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, catalogItemId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SubscriptionResponse')
          ..add('catalogItemId', catalogItemId)
          ..add('createdAt', createdAt)
          ..add('id', id))
        .toString();
  }
}

class SubscriptionResponseBuilder
    implements Builder<SubscriptionResponse, SubscriptionResponseBuilder> {
  _$SubscriptionResponse? _$v;

  String? _catalogItemId;
  String? get catalogItemId => _$this._catalogItemId;
  set catalogItemId(String? catalogItemId) =>
      _$this._catalogItemId = catalogItemId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  SubscriptionResponseBuilder() {
    SubscriptionResponse._defaults(this);
  }

  SubscriptionResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _catalogItemId = $v.catalogItemId;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SubscriptionResponse other) {
    _$v = other as _$SubscriptionResponse;
  }

  @override
  void update(void Function(SubscriptionResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SubscriptionResponse build() => _build();

  _$SubscriptionResponse _build() {
    final _$result = _$v ??
        _$SubscriptionResponse._(
          catalogItemId: BuiltValueNullFieldError.checkNotNull(
              catalogItemId, r'SubscriptionResponse', 'catalogItemId'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'SubscriptionResponse', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'SubscriptionResponse', 'id'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
