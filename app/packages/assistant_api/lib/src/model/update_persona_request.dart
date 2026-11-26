//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_persona_request.g.dart';

/// Body for `PATCH /api/personas/{id}`.
///
/// Properties:
/// * [starterPrompts] - Replaces the starter prompts: at most 4, each 1 to 200 characters after trimming. Absent leaves them unchanged; `[]` clears them.
@BuiltValue()
abstract class UpdatePersonaRequest
    implements Built<UpdatePersonaRequest, UpdatePersonaRequestBuilder> {
  /// Replaces the starter prompts: at most 4, each 1 to 200 characters after trimming. Absent leaves them unchanged; `[]` clears them.
  @BuiltValueField(wireName: r'starter_prompts')
  BuiltList<String>? get starterPrompts;

  UpdatePersonaRequest._();

  factory UpdatePersonaRequest([void updates(UpdatePersonaRequestBuilder b)]) =
      _$UpdatePersonaRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdatePersonaRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdatePersonaRequest> get serializer =>
      _$UpdatePersonaRequestSerializer();
}

class _$UpdatePersonaRequestSerializer
    implements PrimitiveSerializer<UpdatePersonaRequest> {
  @override
  final Iterable<Type> types = const [
    UpdatePersonaRequest,
    _$UpdatePersonaRequest
  ];

  @override
  final String wireName = r'UpdatePersonaRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdatePersonaRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.starterPrompts != null) {
      yield r'starter_prompts';
      yield serializers.serialize(
        object.starterPrompts,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdatePersonaRequest object, {
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
    required UpdatePersonaRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'starter_prompts':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.starterPrompts.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdatePersonaRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdatePersonaRequestBuilder();
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
