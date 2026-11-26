// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'edit_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$EditMessageRequest extends EditMessageRequest {
  @override
  final String message;

  factory _$EditMessageRequest(
          [void Function(EditMessageRequestBuilder)? updates]) =>
      (EditMessageRequestBuilder()..update(updates))._build();

  _$EditMessageRequest._({required this.message}) : super._();
  @override
  EditMessageRequest rebuild(
          void Function(EditMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  EditMessageRequestBuilder toBuilder() =>
      EditMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is EditMessageRequest && message == other.message;
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
    return (newBuiltValueToStringHelper(r'EditMessageRequest')
          ..add('message', message))
        .toString();
  }
}

class EditMessageRequestBuilder
    implements Builder<EditMessageRequest, EditMessageRequestBuilder> {
  _$EditMessageRequest? _$v;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  EditMessageRequestBuilder() {
    EditMessageRequest._defaults(this);
  }

  EditMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _message = $v.message;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(EditMessageRequest other) {
    _$v = other as _$EditMessageRequest;
  }

  @override
  void update(void Function(EditMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  EditMessageRequest build() => _build();

  _$EditMessageRequest _build() {
    final _$result = _$v ??
        _$EditMessageRequest._(
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'EditMessageRequest', 'message'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
