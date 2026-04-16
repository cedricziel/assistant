// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'api_send_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApiSendMessageRequest extends ApiSendMessageRequest {
  @override
  final BuiltList<String>? attachmentIds;
  @override
  final String message;

  factory _$ApiSendMessageRequest(
          [void Function(ApiSendMessageRequestBuilder)? updates]) =>
      (ApiSendMessageRequestBuilder()..update(updates))._build();

  _$ApiSendMessageRequest._({this.attachmentIds, required this.message})
      : super._();
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
    return other is ApiSendMessageRequest &&
        attachmentIds == other.attachmentIds &&
        message == other.message;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, attachmentIds.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApiSendMessageRequest')
          ..add('attachmentIds', attachmentIds)
          ..add('message', message))
        .toString();
  }
}

class ApiSendMessageRequestBuilder
    implements Builder<ApiSendMessageRequest, ApiSendMessageRequestBuilder> {
  _$ApiSendMessageRequest? _$v;

  ListBuilder<String>? _attachmentIds;
  ListBuilder<String> get attachmentIds =>
      _$this._attachmentIds ??= ListBuilder<String>();
  set attachmentIds(ListBuilder<String>? attachmentIds) =>
      _$this._attachmentIds = attachmentIds;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  ApiSendMessageRequestBuilder() {
    ApiSendMessageRequest._defaults(this);
  }

  ApiSendMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _attachmentIds = $v.attachmentIds?.toBuilder();
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
    _$ApiSendMessageRequest _$result;
    try {
      _$result = _$v ??
          _$ApiSendMessageRequest._(
            attachmentIds: _attachmentIds?.build(),
            message: BuiltValueNullFieldError.checkNotNull(
                message, r'ApiSendMessageRequest', 'message'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'attachmentIds';
        _attachmentIds?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ApiSendMessageRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
