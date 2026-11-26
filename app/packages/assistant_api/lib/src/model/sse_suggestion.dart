//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_suggestion.g.dart';

/// One follow-up prompt in an [`SseSuggestionsEvent`].
///
/// Properties:
/// * [prompt] - Text to send as the user's next message.
@BuiltValue()
abstract class SseSuggestion
    implements Built<SseSuggestion, SseSuggestionBuilder> {
  /// Text to send as the user's next message.
  @BuiltValueField(wireName: r'prompt')
  String get prompt;

  SseSuggestion._();

  factory SseSuggestion([void updates(SseSuggestionBuilder b)]) =
      _$SseSuggestion;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSuggestionBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSuggestion> get serializer =>
      _$SseSuggestionSerializer();
}

class _$SseSuggestionSerializer implements PrimitiveSerializer<SseSuggestion> {
  @override
  final Iterable<Type> types = const [SseSuggestion, _$SseSuggestion];

  @override
  final String wireName = r'SseSuggestion';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSuggestion object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'prompt';
    yield serializers.serialize(
      object.prompt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSuggestion object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object,
            specifiedType: specifiedType)
        .toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SseSuggestionBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'prompt':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.prompt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseSuggestion deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSuggestionBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
