// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'quick_message_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$QuickMessageResponse extends QuickMessageResponse {
  @override
  final String answer;
  @override
  final String conversationId;
  @override
  final String? messageId;

  factory _$QuickMessageResponse(
          [void Function(QuickMessageResponseBuilder)? updates]) =>
      (QuickMessageResponseBuilder()..update(updates))._build();

  _$QuickMessageResponse._(
      {required this.answer, required this.conversationId, this.messageId})
      : super._();
  @override
  QuickMessageResponse rebuild(
          void Function(QuickMessageResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  QuickMessageResponseBuilder toBuilder() =>
      QuickMessageResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is QuickMessageResponse &&
        answer == other.answer &&
        conversationId == other.conversationId &&
        messageId == other.messageId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, answer.hashCode);
    _$hash = $jc(_$hash, conversationId.hashCode);
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'QuickMessageResponse')
          ..add('answer', answer)
          ..add('conversationId', conversationId)
          ..add('messageId', messageId))
        .toString();
  }
}

class QuickMessageResponseBuilder
    implements Builder<QuickMessageResponse, QuickMessageResponseBuilder> {
  _$QuickMessageResponse? _$v;

  String? _answer;
  String? get answer => _$this._answer;
  set answer(String? answer) => _$this._answer = answer;

  String? _conversationId;
  String? get conversationId => _$this._conversationId;
  set conversationId(String? conversationId) =>
      _$this._conversationId = conversationId;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  QuickMessageResponseBuilder() {
    QuickMessageResponse._defaults(this);
  }

  QuickMessageResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _answer = $v.answer;
      _conversationId = $v.conversationId;
      _messageId = $v.messageId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(QuickMessageResponse other) {
    _$v = other as _$QuickMessageResponse;
  }

  @override
  void update(void Function(QuickMessageResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  QuickMessageResponse build() => _build();

  _$QuickMessageResponse _build() {
    final _$result = _$v ??
        _$QuickMessageResponse._(
          answer: BuiltValueNullFieldError.checkNotNull(
              answer, r'QuickMessageResponse', 'answer'),
          conversationId: BuiltValueNullFieldError.checkNotNull(
              conversationId, r'QuickMessageResponse', 'conversationId'),
          messageId: messageId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
