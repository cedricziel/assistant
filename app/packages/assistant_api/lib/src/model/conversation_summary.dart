//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'conversation_summary.g.dart';

/// A conversation summary (no messages).
///
/// Properties:
/// * [archivedAt] - When the conversation was archived; absent when it is not archived.
/// * [createdAt]
/// * [id]
/// * [pinnedAt] - When the conversation was pinned; absent when it is not pinned.
/// * [title]
/// * [updatedAt]
@BuiltValue()
abstract class ConversationSummary
    implements Built<ConversationSummary, ConversationSummaryBuilder> {
  /// When the conversation was archived; absent when it is not archived.
  @BuiltValueField(wireName: r'archived_at')
  DateTime? get archivedAt;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  /// When the conversation was pinned; absent when it is not pinned.
  @BuiltValueField(wireName: r'pinned_at')
  DateTime? get pinnedAt;

  @BuiltValueField(wireName: r'title')
  String get title;

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  ConversationSummary._();

  factory ConversationSummary([void updates(ConversationSummaryBuilder b)]) =
      _$ConversationSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConversationSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConversationSummary> get serializer =>
      _$ConversationSummarySerializer();
}

class _$ConversationSummarySerializer
    implements PrimitiveSerializer<ConversationSummary> {
  @override
  final Iterable<Type> types = const [
    ConversationSummary,
    _$ConversationSummary
  ];

  @override
  final String wireName = r'ConversationSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConversationSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.archivedAt != null) {
      yield r'archived_at';
      yield serializers.serialize(
        object.archivedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    if (object.pinnedAt != null) {
      yield r'pinned_at';
      yield serializers.serialize(
        object.pinnedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
    yield r'title';
    yield serializers.serialize(
      object.title,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(DateTime),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ConversationSummary object, {
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
    required ConversationSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'archived_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.archivedAt = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'pinned_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.pinnedAt = valueDes;
          break;
        case r'title':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.title = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
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
  ConversationSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConversationSummaryBuilder();
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
