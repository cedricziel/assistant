//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/persona_file_slot.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'persona_detail.g.dart';

/// Full persona detail including file slot inventory.
///
/// Properties:
/// * [createdAt] 
/// * [files] 
/// * [id] 
/// * [isDefault] 
/// * [name] 
/// * [skillAccessMode] 
/// * [turnTimeoutSecs] 
/// * [updatedAt] 
@BuiltValue()
abstract class PersonaDetail implements Built<PersonaDetail, PersonaDetailBuilder> {
  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'files')
  BuiltList<PersonaFileSlot> get files;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'is_default')
  bool get isDefault;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'skill_access_mode')
  String get skillAccessMode;

  @BuiltValueField(wireName: r'turn_timeout_secs')
  int? get turnTimeoutSecs;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  PersonaDetail._();

  factory PersonaDetail([void updates(PersonaDetailBuilder b)]) = _$PersonaDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PersonaDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PersonaDetail> get serializer => _$PersonaDetailSerializer();
}

class _$PersonaDetailSerializer implements PrimitiveSerializer<PersonaDetail> {
  @override
  final Iterable<Type> types = const [PersonaDetail, _$PersonaDetail];

  @override
  final String wireName = r'PersonaDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PersonaDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'files';
    yield serializers.serialize(
      object.files,
      specifiedType: const FullType(BuiltList, [FullType(PersonaFileSlot)]),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'is_default';
    yield serializers.serialize(
      object.isDefault,
      specifiedType: const FullType(bool),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'skill_access_mode';
    yield serializers.serialize(
      object.skillAccessMode,
      specifiedType: const FullType(String),
    );
    if (object.turnTimeoutSecs != null) {
      yield r'turn_timeout_secs';
      yield serializers.serialize(
        object.turnTimeoutSecs,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PersonaDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PersonaDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'files':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(PersonaFileSlot)]),
          ) as BuiltList<PersonaFileSlot>;
          result.files.replace(valueDes);
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'is_default':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.isDefault = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'skill_access_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.skillAccessMode = valueDes;
          break;
        case r'turn_timeout_secs':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.turnTimeoutSecs = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PersonaDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PersonaDetailBuilder();
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

