// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'subscribe_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SubscribeRequest extends SubscribeRequest {
  @override
  final String auth;
  @override
  final String endpoint;
  @override
  final String p256dh;

  factory _$SubscribeRequest(
          [void Function(SubscribeRequestBuilder)? updates]) =>
      (SubscribeRequestBuilder()..update(updates))._build();

  _$SubscribeRequest._(
      {required this.auth, required this.endpoint, required this.p256dh})
      : super._();
  @override
  SubscribeRequest rebuild(void Function(SubscribeRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SubscribeRequestBuilder toBuilder() =>
      SubscribeRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SubscribeRequest &&
        auth == other.auth &&
        endpoint == other.endpoint &&
        p256dh == other.p256dh;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, auth.hashCode);
    _$hash = $jc(_$hash, endpoint.hashCode);
    _$hash = $jc(_$hash, p256dh.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SubscribeRequest')
          ..add('auth', auth)
          ..add('endpoint', endpoint)
          ..add('p256dh', p256dh))
        .toString();
  }
}

class SubscribeRequestBuilder
    implements Builder<SubscribeRequest, SubscribeRequestBuilder> {
  _$SubscribeRequest? _$v;

  String? _auth;
  String? get auth => _$this._auth;
  set auth(String? auth) => _$this._auth = auth;

  String? _endpoint;
  String? get endpoint => _$this._endpoint;
  set endpoint(String? endpoint) => _$this._endpoint = endpoint;

  String? _p256dh;
  String? get p256dh => _$this._p256dh;
  set p256dh(String? p256dh) => _$this._p256dh = p256dh;

  SubscribeRequestBuilder() {
    SubscribeRequest._defaults(this);
  }

  SubscribeRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _auth = $v.auth;
      _endpoint = $v.endpoint;
      _p256dh = $v.p256dh;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SubscribeRequest other) {
    _$v = other as _$SubscribeRequest;
  }

  @override
  void update(void Function(SubscribeRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SubscribeRequest build() => _build();

  _$SubscribeRequest _build() {
    final _$result = _$v ??
        _$SubscribeRequest._(
          auth: BuiltValueNullFieldError.checkNotNull(
              auth, r'SubscribeRequest', 'auth'),
          endpoint: BuiltValueNullFieldError.checkNotNull(
              endpoint, r'SubscribeRequest', 'endpoint'),
          p256dh: BuiltValueNullFieldError.checkNotNull(
              p256dh, r'SubscribeRequest', 'p256dh'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
