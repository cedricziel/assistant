//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_active_persona_request.g.dart';

/// Body for `POST /api/personas/active`.
///
/// Properties:
/// * [id] 
@BuiltValue()
abstract class SetActivePersonaRequest implements Built<SetActivePersonaRequest, SetActivePersonaRequestBuilder> {
  @BuiltValueField(wireName: r'id')
  String get id;

  SetActivePersonaRequest._();

  factory SetActivePersonaRequest([void updates(SetActivePersonaRequestBuilder b)]) = _$SetActivePersonaRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetActivePersonaRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetActivePersonaRequest> get serializer => _$SetActivePersonaRequestSerializer();
}

class _$SetActivePersonaRequestSerializer implements PrimitiveSerializer<SetActivePersonaRequest> {
  @override
  final Iterable<Type> types = const [SetActivePersonaRequest, _$SetActivePersonaRequest];

  @override
  final String wireName = r'SetActivePersonaRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetActivePersonaRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetActivePersonaRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SetActivePersonaRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetActivePersonaRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetActivePersonaRequestBuilder();
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

