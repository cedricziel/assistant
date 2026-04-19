//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'persona_file_content.g.dart';

/// Content of a persona file slot.
///
/// Properties:
/// * [content] 
/// * [filename] 
@BuiltValue()
abstract class PersonaFileContent implements Built<PersonaFileContent, PersonaFileContentBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'filename')
  String get filename;

  PersonaFileContent._();

  factory PersonaFileContent([void updates(PersonaFileContentBuilder b)]) = _$PersonaFileContent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PersonaFileContentBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PersonaFileContent> get serializer => _$PersonaFileContentSerializer();
}

class _$PersonaFileContentSerializer implements PrimitiveSerializer<PersonaFileContent> {
  @override
  final Iterable<Type> types = const [PersonaFileContent, _$PersonaFileContent];

  @override
  final String wireName = r'PersonaFileContent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PersonaFileContent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'filename';
    yield serializers.serialize(
      object.filename,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PersonaFileContent object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PersonaFileContentBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        case r'filename':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.filename = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PersonaFileContent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PersonaFileContentBuilder();
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

