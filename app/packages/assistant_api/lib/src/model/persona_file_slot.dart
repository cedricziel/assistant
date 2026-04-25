//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'persona_file_slot.g.dart';

/// A single persona file slot.
///
/// Properties:
/// * [description]
/// * [displayName]
/// * [exists]
/// * [filename]
@BuiltValue()
abstract class PersonaFileSlot
    implements Built<PersonaFileSlot, PersonaFileSlotBuilder> {
  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'display_name')
  String get displayName;

  @BuiltValueField(wireName: r'exists')
  bool get exists;

  @BuiltValueField(wireName: r'filename')
  String get filename;

  PersonaFileSlot._();

  factory PersonaFileSlot([void updates(PersonaFileSlotBuilder b)]) =
      _$PersonaFileSlot;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PersonaFileSlotBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PersonaFileSlot> get serializer =>
      _$PersonaFileSlotSerializer();
}

class _$PersonaFileSlotSerializer
    implements PrimitiveSerializer<PersonaFileSlot> {
  @override
  final Iterable<Type> types = const [PersonaFileSlot, _$PersonaFileSlot];

  @override
  final String wireName = r'PersonaFileSlot';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PersonaFileSlot object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'display_name';
    yield serializers.serialize(
      object.displayName,
      specifiedType: const FullType(String),
    );
    yield r'exists';
    yield serializers.serialize(
      object.exists,
      specifiedType: const FullType(bool),
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
    PersonaFileSlot object, {
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
    required PersonaFileSlotBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'display_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.displayName = valueDes;
          break;
        case r'exists':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.exists = valueDes;
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
  PersonaFileSlot deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PersonaFileSlotBuilder();
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
