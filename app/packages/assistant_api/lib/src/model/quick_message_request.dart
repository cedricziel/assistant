//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'quick_message_request.g.dart';

/// Body for `POST /api/quick-message`.
///
/// Properties:
/// * [context] - Optional context text to include with the message (e.g. clipboard contents, file text).  Prepended to the user message for the LLM.
/// * [message] - The message text to send to the assistant.
/// * [personaId] - Optional persona ID to route the message to a specific persona. When absent, the server's active persona is used.
@BuiltValue()
abstract class QuickMessageRequest
    implements Built<QuickMessageRequest, QuickMessageRequestBuilder> {
  /// Optional context text to include with the message (e.g. clipboard contents, file text).  Prepended to the user message for the LLM.
  @BuiltValueField(wireName: r'context')
  String? get context;

  /// The message text to send to the assistant.
  @BuiltValueField(wireName: r'message')
  String get message;

  /// Optional persona ID to route the message to a specific persona. When absent, the server's active persona is used.
  @BuiltValueField(wireName: r'persona_id')
  String? get personaId;

  QuickMessageRequest._();

  factory QuickMessageRequest([void updates(QuickMessageRequestBuilder b)]) =
      _$QuickMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(QuickMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<QuickMessageRequest> get serializer =>
      _$QuickMessageRequestSerializer();
}

class _$QuickMessageRequestSerializer
    implements PrimitiveSerializer<QuickMessageRequest> {
  @override
  final Iterable<Type> types = const [
    QuickMessageRequest,
    _$QuickMessageRequest
  ];

  @override
  final String wireName = r'QuickMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    QuickMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.context != null) {
      yield r'context';
      yield serializers.serialize(
        object.context,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    if (object.personaId != null) {
      yield r'persona_id';
      yield serializers.serialize(
        object.personaId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    QuickMessageRequest object, {
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
    required QuickMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'context':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.context = valueDes;
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'persona_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.personaId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  QuickMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = QuickMessageRequestBuilder();
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
