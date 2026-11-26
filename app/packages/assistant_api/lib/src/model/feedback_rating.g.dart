// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'feedback_rating.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const FeedbackRating _$positive = const FeedbackRating._('positive');
const FeedbackRating _$negative = const FeedbackRating._('negative');
const FeedbackRating _$unknownDefaultOpenApi =
    const FeedbackRating._('unknownDefaultOpenApi');

FeedbackRating _$valueOf(String name) {
  switch (name) {
    case 'positive':
      return _$positive;
    case 'negative':
      return _$negative;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<FeedbackRating> _$values =
    BuiltSet<FeedbackRating>(const <FeedbackRating>[
  _$positive,
  _$negative,
  _$unknownDefaultOpenApi,
]);

class _$FeedbackRatingMeta {
  const _$FeedbackRatingMeta();
  FeedbackRating get positive => _$positive;
  FeedbackRating get negative => _$negative;
  FeedbackRating get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  FeedbackRating valueOf(String name) => _$valueOf(name);
  BuiltSet<FeedbackRating> get values => _$values;
}

abstract class _$FeedbackRatingMixin {
  // ignore: non_constant_identifier_names
  _$FeedbackRatingMeta get FeedbackRating => const _$FeedbackRatingMeta();
}

Serializer<FeedbackRating> _$feedbackRatingSerializer =
    _$FeedbackRatingSerializer();

class _$FeedbackRatingSerializer
    implements PrimitiveSerializer<FeedbackRating> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'positive': 'positive',
    'negative': 'negative',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'positive': 'positive',
    'negative': 'negative',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[FeedbackRating];
  @override
  final String wireName = 'FeedbackRating';

  @override
  Object serialize(Serializers serializers, FeedbackRating object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  FeedbackRating deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      FeedbackRating.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
