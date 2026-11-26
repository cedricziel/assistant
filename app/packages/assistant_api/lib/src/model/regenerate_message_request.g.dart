// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'regenerate_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegenerateMessageRequest extends RegenerateMessageRequest {
  @override
  final String? model;

  factory _$RegenerateMessageRequest(
          [void Function(RegenerateMessageRequestBuilder)? updates]) =>
      (RegenerateMessageRequestBuilder()..update(updates))._build();

  _$RegenerateMessageRequest._({this.model}) : super._();
  @override
  RegenerateMessageRequest rebuild(
          void Function(RegenerateMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RegenerateMessageRequestBuilder toBuilder() =>
      RegenerateMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegenerateMessageRequest && model == other.model;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, model.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegenerateMessageRequest')
          ..add('model', model))
        .toString();
  }
}

class RegenerateMessageRequestBuilder
    implements
        Builder<RegenerateMessageRequest, RegenerateMessageRequestBuilder> {
  _$RegenerateMessageRequest? _$v;

  String? _model;
  String? get model => _$this._model;
  set model(String? model) => _$this._model = model;

  RegenerateMessageRequestBuilder() {
    RegenerateMessageRequest._defaults(this);
  }

  RegenerateMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _model = $v.model;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegenerateMessageRequest other) {
    _$v = other as _$RegenerateMessageRequest;
  }

  @override
  void update(void Function(RegenerateMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RegenerateMessageRequest build() => _build();

  _$RegenerateMessageRequest _build() {
    final _$result = _$v ??
        _$RegenerateMessageRequest._(
          model: model,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
