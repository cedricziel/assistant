// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_feedback_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageFeedbackResponse extends MessageFeedbackResponse {
  @override
  final String? comment;
  @override
  final DateTime createdAt;
  @override
  final String messageId;
  @override
  final FeedbackRating rating;
  @override
  final DateTime updatedAt;

  factory _$MessageFeedbackResponse(
          [void Function(MessageFeedbackResponseBuilder)? updates]) =>
      (MessageFeedbackResponseBuilder()..update(updates))._build();

  _$MessageFeedbackResponse._(
      {this.comment,
      required this.createdAt,
      required this.messageId,
      required this.rating,
      required this.updatedAt})
      : super._();
  @override
  MessageFeedbackResponse rebuild(
          void Function(MessageFeedbackResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageFeedbackResponseBuilder toBuilder() =>
      MessageFeedbackResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MessageFeedbackResponse &&
        comment == other.comment &&
        createdAt == other.createdAt &&
        messageId == other.messageId &&
        rating == other.rating &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, comment.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jc(_$hash, rating.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageFeedbackResponse')
          ..add('comment', comment)
          ..add('createdAt', createdAt)
          ..add('messageId', messageId)
          ..add('rating', rating)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class MessageFeedbackResponseBuilder
    implements
        Builder<MessageFeedbackResponse, MessageFeedbackResponseBuilder> {
  _$MessageFeedbackResponse? _$v;

  String? _comment;
  String? get comment => _$this._comment;
  set comment(String? comment) => _$this._comment = comment;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  FeedbackRating? _rating;
  FeedbackRating? get rating => _$this._rating;
  set rating(FeedbackRating? rating) => _$this._rating = rating;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  MessageFeedbackResponseBuilder() {
    MessageFeedbackResponse._defaults(this);
  }

  MessageFeedbackResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _comment = $v.comment;
      _createdAt = $v.createdAt;
      _messageId = $v.messageId;
      _rating = $v.rating;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MessageFeedbackResponse other) {
    _$v = other as _$MessageFeedbackResponse;
  }

  @override
  void update(void Function(MessageFeedbackResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MessageFeedbackResponse build() => _build();

  _$MessageFeedbackResponse _build() {
    final _$result = _$v ??
        _$MessageFeedbackResponse._(
          comment: comment,
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'MessageFeedbackResponse', 'createdAt'),
          messageId: BuiltValueNullFieldError.checkNotNull(
              messageId, r'MessageFeedbackResponse', 'messageId'),
          rating: BuiltValueNullFieldError.checkNotNull(
              rating, r'MessageFeedbackResponse', 'rating'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'MessageFeedbackResponse', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
