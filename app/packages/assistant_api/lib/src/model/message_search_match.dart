//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/search_segment.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message_search_match.g.dart';

/// A message that matched a search query.
///
/// Properties:
/// * [createdAt]
/// * [messageId]
/// * [role] - `user` or `assistant`.
/// * [segments] - An excerpt around the hits; elided text is marked with `…`.
@BuiltValue()
abstract class MessageSearchMatch
    implements Built<MessageSearchMatch, MessageSearchMatchBuilder> {
  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'message_id')
  String get messageId;

  /// `user` or `assistant`.
  @BuiltValueField(wireName: r'role')
  String get role;

  /// An excerpt around the hits; elided text is marked with `…`.
  @BuiltValueField(wireName: r'segments')
  BuiltList<SearchSegment> get segments;

  MessageSearchMatch._();

  factory MessageSearchMatch([void updates(MessageSearchMatchBuilder b)]) =
      _$MessageSearchMatch;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageSearchMatchBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MessageSearchMatch> get serializer =>
      _$MessageSearchMatchSerializer();
}

class _$MessageSearchMatchSerializer
    implements PrimitiveSerializer<MessageSearchMatch> {
  @override
  final Iterable<Type> types = const [MessageSearchMatch, _$MessageSearchMatch];

  @override
  final String wireName = r'MessageSearchMatch';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MessageSearchMatch object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'message_id';
    yield serializers.serialize(
      object.messageId,
      specifiedType: const FullType(String),
    );
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    yield r'segments';
    yield serializers.serialize(
      object.segments,
      specifiedType: const FullType(BuiltList, [FullType(SearchSegment)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MessageSearchMatch object, {
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
    required MessageSearchMatchBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.messageId = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'segments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SearchSegment)]),
          ) as BuiltList<SearchSegment>;
          result.segments.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MessageSearchMatch deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageSearchMatchBuilder();
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
