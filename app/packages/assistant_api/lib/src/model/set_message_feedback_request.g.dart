// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_message_feedback_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetMessageFeedbackRequest extends SetMessageFeedbackRequest {
  @override
  final String? comment;
  @override
  final FeedbackRating rating;

  factory _$SetMessageFeedbackRequest(
          [void Function(SetMessageFeedbackRequestBuilder)? updates]) =>
      (SetMessageFeedbackRequestBuilder()..update(updates))._build();

  _$SetMessageFeedbackRequest._({this.comment, required this.rating})
      : super._();
  @override
  SetMessageFeedbackRequest rebuild(
          void Function(SetMessageFeedbackRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetMessageFeedbackRequestBuilder toBuilder() =>
      SetMessageFeedbackRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetMessageFeedbackRequest &&
        comment == other.comment &&
        rating == other.rating;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, comment.hashCode);
    _$hash = $jc(_$hash, rating.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetMessageFeedbackRequest')
          ..add('comment', comment)
          ..add('rating', rating))
        .toString();
  }
}

class SetMessageFeedbackRequestBuilder
    implements
        Builder<SetMessageFeedbackRequest, SetMessageFeedbackRequestBuilder> {
  _$SetMessageFeedbackRequest? _$v;

  String? _comment;
  String? get comment => _$this._comment;
  set comment(String? comment) => _$this._comment = comment;

  FeedbackRating? _rating;
  FeedbackRating? get rating => _$this._rating;
  set rating(FeedbackRating? rating) => _$this._rating = rating;

  SetMessageFeedbackRequestBuilder() {
    SetMessageFeedbackRequest._defaults(this);
  }

  SetMessageFeedbackRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _comment = $v.comment;
      _rating = $v.rating;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetMessageFeedbackRequest other) {
    _$v = other as _$SetMessageFeedbackRequest;
  }

  @override
  void update(void Function(SetMessageFeedbackRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetMessageFeedbackRequest build() => _build();

  _$SetMessageFeedbackRequest _build() {
    final _$result = _$v ??
        _$SetMessageFeedbackRequest._(
          comment: comment,
          rating: BuiltValueNullFieldError.checkNotNull(
              rating, r'SetMessageFeedbackRequest', 'rating'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
