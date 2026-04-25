// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_subscription_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateSubscriptionRequest extends CreateSubscriptionRequest {
  @override
  final String catalogItemId;

  factory _$CreateSubscriptionRequest(
          [void Function(CreateSubscriptionRequestBuilder)? updates]) =>
      (CreateSubscriptionRequestBuilder()..update(updates))._build();

  _$CreateSubscriptionRequest._({required this.catalogItemId}) : super._();
  @override
  CreateSubscriptionRequest rebuild(
          void Function(CreateSubscriptionRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateSubscriptionRequestBuilder toBuilder() =>
      CreateSubscriptionRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateSubscriptionRequest &&
        catalogItemId == other.catalogItemId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, catalogItemId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateSubscriptionRequest')
          ..add('catalogItemId', catalogItemId))
        .toString();
  }
}

class CreateSubscriptionRequestBuilder
    implements
        Builder<CreateSubscriptionRequest, CreateSubscriptionRequestBuilder> {
  _$CreateSubscriptionRequest? _$v;

  String? _catalogItemId;
  String? get catalogItemId => _$this._catalogItemId;
  set catalogItemId(String? catalogItemId) =>
      _$this._catalogItemId = catalogItemId;

  CreateSubscriptionRequestBuilder() {
    CreateSubscriptionRequest._defaults(this);
  }

  CreateSubscriptionRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _catalogItemId = $v.catalogItemId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateSubscriptionRequest other) {
    _$v = other as _$CreateSubscriptionRequest;
  }

  @override
  void update(void Function(CreateSubscriptionRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateSubscriptionRequest build() => _build();

  _$CreateSubscriptionRequest _build() {
    final _$result = _$v ??
        _$CreateSubscriptionRequest._(
          catalogItemId: BuiltValueNullFieldError.checkNotNull(
              catalogItemId, r'CreateSubscriptionRequest', 'catalogItemId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
