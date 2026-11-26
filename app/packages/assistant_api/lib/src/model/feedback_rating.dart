//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'feedback_rating.g.dart';

class FeedbackRating extends EnumClass {
  /// The caller's verdict on an assistant message.
  @BuiltValueEnumConst(wireName: r'positive')
  static const FeedbackRating positive = _$positive;

  /// The caller's verdict on an assistant message.
  @BuiltValueEnumConst(wireName: r'negative')
  static const FeedbackRating negative = _$negative;

  /// The caller's verdict on an assistant message.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const FeedbackRating unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<FeedbackRating> get serializer =>
      _$feedbackRatingSerializer;

  const FeedbackRating._(String name) : super(name);

  static BuiltSet<FeedbackRating> get values => _$values;
  static FeedbackRating valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class FeedbackRatingMixin = Object with _$FeedbackRatingMixin;
