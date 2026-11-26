//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/interaction_outcome_view.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_interaction_resolved_event.g.dart';

/// SSE `interaction_resolved` event: exactly one per interaction of a live run.
///
/// Properties:
/// * [interactionId]
/// * [kind]
/// * [outcome]
/// * [resolvedAt]
/// * [response] - The owner's answer. Only when `outcome` is `resolved`.
/// * [ruleId] - Id of the permission rule an \"always\" answer saved.
@BuiltValue()
abstract class SseInteractionResolvedEvent
    implements
        Built<SseInteractionResolvedEvent, SseInteractionResolvedEventBuilder> {
  @BuiltValueField(wireName: r'interaction_id')
  String get interactionId;

  @BuiltValueField(wireName: r'kind')
  String get kind;

  @BuiltValueField(wireName: r'outcome')
  InteractionOutcomeView get outcome;
  // enum outcomeEnum {  resolved,  cancelled,  expired,  };

  @BuiltValueField(wireName: r'resolved_at')
  DateTime get resolvedAt;

  /// The owner's answer. Only when `outcome` is `resolved`.
  @BuiltValueField(wireName: r'response')
  JsonObject? get response;

  /// Id of the permission rule an \"always\" answer saved.
  @BuiltValueField(wireName: r'rule_id')
  String? get ruleId;

  SseInteractionResolvedEvent._();

  factory SseInteractionResolvedEvent(
          [void updates(SseInteractionResolvedEventBuilder b)]) =
      _$SseInteractionResolvedEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseInteractionResolvedEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseInteractionResolvedEvent> get serializer =>
      _$SseInteractionResolvedEventSerializer();
}

class _$SseInteractionResolvedEventSerializer
    implements PrimitiveSerializer<SseInteractionResolvedEvent> {
  @override
  final Iterable<Type> types = const [
    SseInteractionResolvedEvent,
    _$SseInteractionResolvedEvent
  ];

  @override
  final String wireName = r'SseInteractionResolvedEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseInteractionResolvedEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'interaction_id';
    yield serializers.serialize(
      object.interactionId,
      specifiedType: const FullType(String),
    );
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(String),
    );
    yield r'outcome';
    yield serializers.serialize(
      object.outcome,
      specifiedType: const FullType(InteractionOutcomeView),
    );
    yield r'resolved_at';
    yield serializers.serialize(
      object.resolvedAt,
      specifiedType: const FullType(DateTime),
    );
    if (object.response != null) {
      yield r'response';
      yield serializers.serialize(
        object.response,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    if (object.ruleId != null) {
      yield r'rule_id';
      yield serializers.serialize(
        object.ruleId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SseInteractionResolvedEvent object, {
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
    required SseInteractionResolvedEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'interaction_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.interactionId = valueDes;
          break;
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.kind = valueDes;
          break;
        case r'outcome':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(InteractionOutcomeView),
          ) as InteractionOutcomeView;
          result.outcome = valueDes;
          break;
        case r'resolved_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.resolvedAt = valueDes;
          break;
        case r'response':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.response = valueDes;
          break;
        case r'rule_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.ruleId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseInteractionResolvedEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseInteractionResolvedEventBuilder();
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
