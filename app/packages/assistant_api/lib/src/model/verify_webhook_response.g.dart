// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verify_webhook_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$VerifyWebhookResponse extends VerifyWebhookResponse {
  @override
  final String detail;
  @override
  final bool success;

  factory _$VerifyWebhookResponse(
          [void Function(VerifyWebhookResponseBuilder)? updates]) =>
      (VerifyWebhookResponseBuilder()..update(updates))._build();

  _$VerifyWebhookResponse._({required this.detail, required this.success})
      : super._();
  @override
  VerifyWebhookResponse rebuild(
          void Function(VerifyWebhookResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  VerifyWebhookResponseBuilder toBuilder() =>
      VerifyWebhookResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is VerifyWebhookResponse &&
        detail == other.detail &&
        success == other.success;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, detail.hashCode);
    _$hash = $jc(_$hash, success.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'VerifyWebhookResponse')
          ..add('detail', detail)
          ..add('success', success))
        .toString();
  }
}

class VerifyWebhookResponseBuilder
    implements Builder<VerifyWebhookResponse, VerifyWebhookResponseBuilder> {
  _$VerifyWebhookResponse? _$v;

  String? _detail;
  String? get detail => _$this._detail;
  set detail(String? detail) => _$this._detail = detail;

  bool? _success;
  bool? get success => _$this._success;
  set success(bool? success) => _$this._success = success;

  VerifyWebhookResponseBuilder() {
    VerifyWebhookResponse._defaults(this);
  }

  VerifyWebhookResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _detail = $v.detail;
      _success = $v.success;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(VerifyWebhookResponse other) {
    _$v = other as _$VerifyWebhookResponse;
  }

  @override
  void update(void Function(VerifyWebhookResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  VerifyWebhookResponse build() => _build();

  _$VerifyWebhookResponse _build() {
    final _$result = _$v ??
        _$VerifyWebhookResponse._(
          detail: BuiltValueNullFieldError.checkNotNull(
              detail, r'VerifyWebhookResponse', 'detail'),
          success: BuiltValueNullFieldError.checkNotNull(
              success, r'VerifyWebhookResponse', 'success'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
