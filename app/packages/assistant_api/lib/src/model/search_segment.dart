//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'search_segment.g.dart';

/// A run of text in a search result, flagged when it matched a query token.  Concatenating the `text` of a list of segments yields the displayed string.
///
/// Properties:
/// * [highlighted]
/// * [text]
@BuiltValue()
abstract class SearchSegment
    implements Built<SearchSegment, SearchSegmentBuilder> {
  @BuiltValueField(wireName: r'highlighted')
  bool get highlighted;

  @BuiltValueField(wireName: r'text')
  String get text;

  SearchSegment._();

  factory SearchSegment([void updates(SearchSegmentBuilder b)]) =
      _$SearchSegment;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SearchSegmentBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SearchSegment> get serializer =>
      _$SearchSegmentSerializer();
}

class _$SearchSegmentSerializer implements PrimitiveSerializer<SearchSegment> {
  @override
  final Iterable<Type> types = const [SearchSegment, _$SearchSegment];

  @override
  final String wireName = r'SearchSegment';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SearchSegment object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'highlighted';
    yield serializers.serialize(
      object.highlighted,
      specifiedType: const FullType(bool),
    );
    yield r'text';
    yield serializers.serialize(
      object.text,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SearchSegment object, {
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
    required SearchSegmentBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'highlighted':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.highlighted = valueDes;
          break;
        case r'text':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.text = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SearchSegment deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SearchSegmentBuilder();
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
