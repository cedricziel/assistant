//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/message_search_match.dart';
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/search_segment.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'conversation_search_summary.g.dart';

/// A conversation matched by a search: the [`ConversationSummary`] fields plus highlighted title and message excerpts.  The summary fields are repeated rather than flattened so the generated client keeps `ConversationSummary` a plain instantiable model.
///
/// Properties:
/// * [createdAt]
/// * [id]
/// * [matches] - Up to 3 matching messages, best first; empty when only the title matched.
/// * [pinnedAt] - When the conversation was pinned; absent when it is not pinned.
/// * [title]
/// * [titleSegments] - The title split into highlighted and plain runs.
/// * [updatedAt]
@BuiltValue()
abstract class ConversationSearchSummary
    implements
        Built<ConversationSearchSummary, ConversationSearchSummaryBuilder> {
  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  /// Up to 3 matching messages, best first; empty when only the title matched.
  @BuiltValueField(wireName: r'matches')
  BuiltList<MessageSearchMatch> get matches;

  /// When the conversation was pinned; absent when it is not pinned.
  @BuiltValueField(wireName: r'pinned_at')
  DateTime? get pinnedAt;

  @BuiltValueField(wireName: r'title')
  String get title;

  /// The title split into highlighted and plain runs.
  @BuiltValueField(wireName: r'title_segments')
  BuiltList<SearchSegment> get titleSegments;

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  ConversationSearchSummary._();

  factory ConversationSearchSummary(
          [void updates(ConversationSearchSummaryBuilder b)]) =
      _$ConversationSearchSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConversationSearchSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConversationSearchSummary> get serializer =>
      _$ConversationSearchSummarySerializer();
}

class _$ConversationSearchSummarySerializer
    implements PrimitiveSerializer<ConversationSearchSummary> {
  @override
  final Iterable<Type> types = const [
    ConversationSearchSummary,
    _$ConversationSearchSummary
  ];

  @override
  final String wireName = r'ConversationSearchSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConversationSearchSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    yield r'matches';
    yield serializers.serialize(
      object.matches,
      specifiedType: const FullType(BuiltList, [FullType(MessageSearchMatch)]),
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
    yield r'title_segments';
    yield serializers.serialize(
      object.titleSegments,
      specifiedType: const FullType(BuiltList, [FullType(SearchSegment)]),
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
    ConversationSearchSummary object, {
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
    required ConversationSearchSummaryBuilder result,
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
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'matches':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(MessageSearchMatch)]),
          ) as BuiltList<MessageSearchMatch>;
          result.matches.replace(valueDes);
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
        case r'title_segments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SearchSegment)]),
          ) as BuiltList<SearchSegment>;
          result.titleSegments.replace(valueDes);
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
  ConversationSearchSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConversationSearchSummaryBuilder();
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
