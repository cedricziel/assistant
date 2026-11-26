//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_conversation_request.g.dart';

/// Body for `PATCH /api/conversations/{id}`. At least one field is required.
///
/// Properties:
/// * [pinned] - `true` pins the conversation, `false` unpins it.
/// * [title] - New title. Trimmed; must be 1 to 200 characters after trimming.
@BuiltValue()
abstract class UpdateConversationRequest
    implements
        Built<UpdateConversationRequest, UpdateConversationRequestBuilder> {
  /// `true` pins the conversation, `false` unpins it.
  @BuiltValueField(wireName: r'pinned')
  bool? get pinned;

  /// New title. Trimmed; must be 1 to 200 characters after trimming.
  @BuiltValueField(wireName: r'title')
  String? get title;

  UpdateConversationRequest._();

  factory UpdateConversationRequest(
          [void updates(UpdateConversationRequestBuilder b)]) =
      _$UpdateConversationRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateConversationRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateConversationRequest> get serializer =>
      _$UpdateConversationRequestSerializer();
}

class _$UpdateConversationRequestSerializer
    implements PrimitiveSerializer<UpdateConversationRequest> {
  @override
  final Iterable<Type> types = const [
    UpdateConversationRequest,
    _$UpdateConversationRequest
  ];

  @override
  final String wireName = r'UpdateConversationRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateConversationRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.pinned != null) {
      yield r'pinned';
      yield serializers.serialize(
        object.pinned,
        specifiedType: const FullType.nullable(bool),
      );
    }
    if (object.title != null) {
      yield r'title';
      yield serializers.serialize(
        object.title,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateConversationRequest object, {
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
    required UpdateConversationRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'pinned':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.pinned = valueDes;
          break;
        case r'title':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.title = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateConversationRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateConversationRequestBuilder();
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
