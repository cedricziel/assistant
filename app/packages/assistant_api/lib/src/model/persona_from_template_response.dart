//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'persona_from_template_response.g.dart';

/// PersonaFromTemplateResponse
///
/// Properties:
/// * [name]
/// * [personaId]
@BuiltValue()
abstract class PersonaFromTemplateResponse
    implements
        Built<PersonaFromTemplateResponse, PersonaFromTemplateResponseBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'persona_id')
  String get personaId;

  PersonaFromTemplateResponse._();

  factory PersonaFromTemplateResponse(
          [void updates(PersonaFromTemplateResponseBuilder b)]) =
      _$PersonaFromTemplateResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PersonaFromTemplateResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PersonaFromTemplateResponse> get serializer =>
      _$PersonaFromTemplateResponseSerializer();
}

class _$PersonaFromTemplateResponseSerializer
    implements PrimitiveSerializer<PersonaFromTemplateResponse> {
  @override
  final Iterable<Type> types = const [
    PersonaFromTemplateResponse,
    _$PersonaFromTemplateResponse
  ];

  @override
  final String wireName = r'PersonaFromTemplateResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PersonaFromTemplateResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'persona_id';
    yield serializers.serialize(
      object.personaId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PersonaFromTemplateResponse object, {
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
    required PersonaFromTemplateResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'persona_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
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
  PersonaFromTemplateResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PersonaFromTemplateResponseBuilder();
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
