//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/sse_suggestion.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_suggestions_event.g.dart';

/// SSE `suggestions` event: tappable follow-up prompts for a finished turn.  Sent only on the run's replay stream, after `done`, when `[suggestions].enabled` is on for the org.
///
/// Properties:
/// * [messageId] - The assistant message the suggestions follow (`done.message_id`).
/// * [suggestions] - Prompts the user could send next, best first.
@BuiltValue()
abstract class SseSuggestionsEvent
    implements Built<SseSuggestionsEvent, SseSuggestionsEventBuilder> {
  /// The assistant message the suggestions follow (`done.message_id`).
  @BuiltValueField(wireName: r'message_id')
  String get messageId;

  /// Prompts the user could send next, best first.
  @BuiltValueField(wireName: r'suggestions')
  BuiltList<SseSuggestion> get suggestions;

  SseSuggestionsEvent._();

  factory SseSuggestionsEvent([void updates(SseSuggestionsEventBuilder b)]) =
      _$SseSuggestionsEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseSuggestionsEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseSuggestionsEvent> get serializer =>
      _$SseSuggestionsEventSerializer();
}

class _$SseSuggestionsEventSerializer
    implements PrimitiveSerializer<SseSuggestionsEvent> {
  @override
  final Iterable<Type> types = const [
    SseSuggestionsEvent,
    _$SseSuggestionsEvent
  ];

  @override
  final String wireName = r'SseSuggestionsEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseSuggestionsEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'message_id';
    yield serializers.serialize(
      object.messageId,
      specifiedType: const FullType(String),
    );
    yield r'suggestions';
    yield serializers.serialize(
      object.suggestions,
      specifiedType: const FullType(BuiltList, [FullType(SseSuggestion)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseSuggestionsEvent object, {
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
    required SseSuggestionsEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.messageId = valueDes;
          break;
        case r'suggestions':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SseSuggestion)]),
          ) as BuiltList<SseSuggestion>;
          result.suggestions.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseSuggestionsEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseSuggestionsEventBuilder();
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
