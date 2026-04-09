// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'unsubscribe_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UnsubscribeRequest extends UnsubscribeRequest {
  @override
  final String endpoint;

  factory _$UnsubscribeRequest(
          [void Function(UnsubscribeRequestBuilder)? updates]) =>
      (UnsubscribeRequestBuilder()..update(updates))._build();

  _$UnsubscribeRequest._({required this.endpoint}) : super._();
  @override
  UnsubscribeRequest rebuild(
          void Function(UnsubscribeRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UnsubscribeRequestBuilder toBuilder() =>
      UnsubscribeRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UnsubscribeRequest && endpoint == other.endpoint;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, endpoint.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UnsubscribeRequest')
          ..add('endpoint', endpoint))
        .toString();
  }
}

class UnsubscribeRequestBuilder
    implements Builder<UnsubscribeRequest, UnsubscribeRequestBuilder> {
  _$UnsubscribeRequest? _$v;

  String? _endpoint;
  String? get endpoint => _$this._endpoint;
  set endpoint(String? endpoint) => _$this._endpoint = endpoint;

  UnsubscribeRequestBuilder() {
    UnsubscribeRequest._defaults(this);
  }

  UnsubscribeRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _endpoint = $v.endpoint;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UnsubscribeRequest other) {
    _$v = other as _$UnsubscribeRequest;
  }

  @override
  void update(void Function(UnsubscribeRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UnsubscribeRequest build() => _build();

  _$UnsubscribeRequest _build() {
    final _$result = _$v ??
        _$UnsubscribeRequest._(
          endpoint: BuiltValueNullFieldError.checkNotNull(
              endpoint, r'UnsubscribeRequest', 'endpoint'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
