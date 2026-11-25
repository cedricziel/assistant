//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/space_access_mode.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'space_access_entry.g.dart';

/// One space row of an org-layer integration.
///
/// Properties:
/// * [mode]
/// * [spaceId]
@BuiltValue()
abstract class SpaceAccessEntry
    implements Built<SpaceAccessEntry, SpaceAccessEntryBuilder> {
  @BuiltValueField(wireName: r'mode')
  SpaceAccessMode get mode;
  // enum modeEnum {  allow,  opt_out,  };

  @BuiltValueField(wireName: r'space_id')
  String get spaceId;

  SpaceAccessEntry._();

  factory SpaceAccessEntry([void updates(SpaceAccessEntryBuilder b)]) =
      _$SpaceAccessEntry;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SpaceAccessEntryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SpaceAccessEntry> get serializer =>
      _$SpaceAccessEntrySerializer();
}

class _$SpaceAccessEntrySerializer
    implements PrimitiveSerializer<SpaceAccessEntry> {
  @override
  final Iterable<Type> types = const [SpaceAccessEntry, _$SpaceAccessEntry];

  @override
  final String wireName = r'SpaceAccessEntry';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SpaceAccessEntry object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'mode';
    yield serializers.serialize(
      object.mode,
      specifiedType: const FullType(SpaceAccessMode),
    );
    yield r'space_id';
    yield serializers.serialize(
      object.spaceId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SpaceAccessEntry object, {
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
    required SpaceAccessEntryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(SpaceAccessMode),
          ) as SpaceAccessMode;
          result.mode = valueDes;
          break;
        case r'space_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.spaceId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SpaceAccessEntry deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SpaceAccessEntryBuilder();
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
