//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'stream_run_events_query.g.dart';

/// StreamRunEventsQuery
///
/// Properties:
/// * [since] - Replay from this sequence number (inclusive). Defaults to 0.
@BuiltValue()
abstract class StreamRunEventsQuery implements Built<StreamRunEventsQuery, StreamRunEventsQueryBuilder> {
  /// Replay from this sequence number (inclusive). Defaults to 0.
  @BuiltValueField(wireName: r'since')
  int? get since;

  StreamRunEventsQuery._();

  factory StreamRunEventsQuery([void updates(StreamRunEventsQueryBuilder b)]) = _$StreamRunEventsQuery;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(StreamRunEventsQueryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<StreamRunEventsQuery> get serializer => _$StreamRunEventsQuerySerializer();
}

class _$StreamRunEventsQuerySerializer implements PrimitiveSerializer<StreamRunEventsQuery> {
  @override
  final Iterable<Type> types = const [StreamRunEventsQuery, _$StreamRunEventsQuery];

  @override
  final String wireName = r'StreamRunEventsQuery';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    StreamRunEventsQuery object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.since != null) {
      yield r'since';
      yield serializers.serialize(
        object.since,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    StreamRunEventsQuery object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required StreamRunEventsQueryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'since':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.since = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  StreamRunEventsQuery deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = StreamRunEventsQueryBuilder();
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

