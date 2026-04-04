//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'string_list.g.dart';

/// A list of strings (used in security requirements).
///
/// Properties:
/// * [list] - The individual string values.
@BuiltValue()
abstract class StringList implements Built<StringList, StringListBuilder> {
  /// The individual string values.
  @BuiltValueField(wireName: r'list')
  BuiltList<String>? get list;

  StringList._();

  factory StringList([void updates(StringListBuilder b)]) = _$StringList;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(StringListBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<StringList> get serializer => _$StringListSerializer();
}

class _$StringListSerializer implements PrimitiveSerializer<StringList> {
  @override
  final Iterable<Type> types = const [StringList, _$StringList];

  @override
  final String wireName = r'StringList';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    StringList object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.list != null) {
      yield r'list';
      yield serializers.serialize(
        object.list,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    StringList object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required StringListBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'list':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.list.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  StringList deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = StringListBuilder();
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

