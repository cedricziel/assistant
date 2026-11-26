// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_feedback_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageFeedbackSummary extends MessageFeedbackSummary {
  @override
  final String? comment;
  @override
  final String rating;
  @override
  final DateTime updatedAt;

  factory _$MessageFeedbackSummary(
          [void Function(MessageFeedbackSummaryBuilder)? updates]) =>
      (MessageFeedbackSummaryBuilder()..update(updates))._build();

  _$MessageFeedbackSummary._(
      {this.comment, required this.rating, required this.updatedAt})
      : super._();
  @override
  MessageFeedbackSummary rebuild(
          void Function(MessageFeedbackSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageFeedbackSummaryBuilder toBuilder() =>
      MessageFeedbackSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MessageFeedbackSummary &&
        comment == other.comment &&
        rating == other.rating &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, comment.hashCode);
    _$hash = $jc(_$hash, rating.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageFeedbackSummary')
          ..add('comment', comment)
          ..add('rating', rating)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class MessageFeedbackSummaryBuilder
    implements Builder<MessageFeedbackSummary, MessageFeedbackSummaryBuilder> {
  _$MessageFeedbackSummary? _$v;

  String? _comment;
  String? get comment => _$this._comment;
  set comment(String? comment) => _$this._comment = comment;

  String? _rating;
  String? get rating => _$this._rating;
  set rating(String? rating) => _$this._rating = rating;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  MessageFeedbackSummaryBuilder() {
    MessageFeedbackSummary._defaults(this);
  }

  MessageFeedbackSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _comment = $v.comment;
      _rating = $v.rating;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MessageFeedbackSummary other) {
    _$v = other as _$MessageFeedbackSummary;
  }

  @override
  void update(void Function(MessageFeedbackSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MessageFeedbackSummary build() => _build();

  _$MessageFeedbackSummary _build() {
    final _$result = _$v ??
        _$MessageFeedbackSummary._(
          comment: comment,
          rating: BuiltValueNullFieldError.checkNotNull(
              rating, r'MessageFeedbackSummary', 'rating'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'MessageFeedbackSummary', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
