// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_send_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApiSendMessageRequest extends ApiSendMessageRequest {
  @override
  final String message;

  factory _$ApiSendMessageRequest(
          [void Function(ApiSendMessageRequestBuilder)? updates]) =>
      (ApiSendMessageRequestBuilder()..update(updates))._build();

  _$ApiSendMessageRequest._({required this.message}) : super._();
  @override
  ApiSendMessageRequest rebuild(
          void Function(ApiSendMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ApiSendMessageRequestBuilder toBuilder() =>
      ApiSendMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApiSendMessageRequest && message == other.message;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApiSendMessageRequest')
          ..add('message', message))
        .toString();
  }
}

class ApiSendMessageRequestBuilder
    implements Builder<ApiSendMessageRequest, ApiSendMessageRequestBuilder> {
  _$ApiSendMessageRequest? _$v;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  ApiSendMessageRequestBuilder() {
    ApiSendMessageRequest._defaults(this);
  }

  ApiSendMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApiSendMessageRequest other) {
    _$v = other as _$ApiSendMessageRequest;
  }

  @override
  void update(void Function(ApiSendMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApiSendMessageRequest build() => _build();

  _$ApiSendMessageRequest _build() {
    final _$result = _$v ??
        _$ApiSendMessageRequest._(
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'ApiSendMessageRequest', 'message'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
