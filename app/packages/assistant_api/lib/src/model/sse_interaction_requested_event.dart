//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_interaction_requested_event.g.dart';

/// SSE `interaction_requested` event: a running turn asks its owner something. The kind-specific request fields sit next to the common ones, as in [`InteractionDetail::request`].
///
/// Properties:
/// * [createdAt]
/// * [expiresAt]
/// * [interactionId]
/// * [kind] - `tool_approval`, `question`, `surface_action`, or a kind a newer server added.
/// * [rendersCard] - Whether the kind renders a card in the timeline.
/// * [toolCallId] - Provider id of the tool call this belongs to, when there is one.
@BuiltValue()
abstract class SseInteractionRequestedEvent
    implements
        Built<SseInteractionRequestedEvent,
            SseInteractionRequestedEventBuilder> {
  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'expires_at')
  DateTime get expiresAt;

  @BuiltValueField(wireName: r'interaction_id')
  String get interactionId;

  /// `tool_approval`, `question`, `surface_action`, or a kind a newer server added.
  @BuiltValueField(wireName: r'kind')
  String get kind;

  /// Whether the kind renders a card in the timeline.
  @BuiltValueField(wireName: r'renders_card')
  bool get rendersCard;

  /// Provider id of the tool call this belongs to, when there is one.
  @BuiltValueField(wireName: r'tool_call_id')
  String? get toolCallId;

  SseInteractionRequestedEvent._();

  factory SseInteractionRequestedEvent(
          [void updates(SseInteractionRequestedEventBuilder b)]) =
      _$SseInteractionRequestedEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseInteractionRequestedEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseInteractionRequestedEvent> get serializer =>
      _$SseInteractionRequestedEventSerializer();
}

class _$SseInteractionRequestedEventSerializer
    implements PrimitiveSerializer<SseInteractionRequestedEvent> {
  @override
  final Iterable<Type> types = const [
    SseInteractionRequestedEvent,
    _$SseInteractionRequestedEvent
  ];

  @override
  final String wireName = r'SseInteractionRequestedEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseInteractionRequestedEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'expires_at';
    yield serializers.serialize(
      object.expiresAt,
      specifiedType: const FullType(DateTime),
    );
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
    yield r'renders_card';
    yield serializers.serialize(
      object.rendersCard,
      specifiedType: const FullType(bool),
    );
    if (object.toolCallId != null) {
      yield r'tool_call_id';
      yield serializers.serialize(
        object.toolCallId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SseInteractionRequestedEvent object, {
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
    required SseInteractionRequestedEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.expiresAt = valueDes;
          break;
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
        case r'renders_card':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.rendersCard = valueDes;
          break;
        case r'tool_call_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.toolCallId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseInteractionRequestedEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseInteractionRequestedEventBuilder();
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
