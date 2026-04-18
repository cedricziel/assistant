// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'quick_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$QuickMessageRequest extends QuickMessageRequest {
  @override
  final String message;

  factory _$QuickMessageRequest(
          [void Function(QuickMessageRequestBuilder)? updates]) =>
      (QuickMessageRequestBuilder()..update(updates))._build();

  _$QuickMessageRequest._({required this.message}) : super._();
  @override
  QuickMessageRequest rebuild(
          void Function(QuickMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  QuickMessageRequestBuilder toBuilder() =>
      QuickMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is QuickMessageRequest && message == other.message;
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
    return (newBuiltValueToStringHelper(r'QuickMessageRequest')
          ..add('message', message))
        .toString();
  }
}

class QuickMessageRequestBuilder
    implements Builder<QuickMessageRequest, QuickMessageRequestBuilder> {
  _$QuickMessageRequest? _$v;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  QuickMessageRequestBuilder() {
    QuickMessageRequest._defaults(this);
  }

  QuickMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(QuickMessageRequest other) {
    _$v = other as _$QuickMessageRequest;
  }

  @override
  void update(void Function(QuickMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  QuickMessageRequest build() => _build();

  _$QuickMessageRequest _build() {
    final _$result = _$v ??
        _$QuickMessageRequest._(
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'QuickMessageRequest', 'message'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
