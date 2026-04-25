//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/span_entry_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'trace_detail_response.g.dart';

/// Full trace detail with span list.
///
/// Properties:
/// * [durationMs]
/// * [personaId]
/// * [spans]
/// * [startTime]
/// * [traceId]
@BuiltValue()
abstract class TraceDetailResponse
    implements Built<TraceDetailResponse, TraceDetailResponseBuilder> {
  @BuiltValueField(wireName: r'duration_ms')
  int get durationMs;

  @BuiltValueField(wireName: r'persona_id')
  String get personaId;

  @BuiltValueField(wireName: r'spans')
  BuiltList<SpanEntryResponse> get spans;

  @BuiltValueField(wireName: r'start_time')
  DateTime get startTime;

  @BuiltValueField(wireName: r'trace_id')
  String get traceId;

  TraceDetailResponse._();

  factory TraceDetailResponse([void updates(TraceDetailResponseBuilder b)]) =
      _$TraceDetailResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TraceDetailResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TraceDetailResponse> get serializer =>
      _$TraceDetailResponseSerializer();
}

class _$TraceDetailResponseSerializer
    implements PrimitiveSerializer<TraceDetailResponse> {
  @override
  final Iterable<Type> types = const [
    TraceDetailResponse,
    _$TraceDetailResponse
  ];

  @override
  final String wireName = r'TraceDetailResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TraceDetailResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'duration_ms';
    yield serializers.serialize(
      object.durationMs,
      specifiedType: const FullType(int),
    );
    yield r'persona_id';
    yield serializers.serialize(
      object.personaId,
      specifiedType: const FullType(String),
    );
    yield r'spans';
    yield serializers.serialize(
      object.spans,
      specifiedType: const FullType(BuiltList, [FullType(SpanEntryResponse)]),
    );
    yield r'start_time';
    yield serializers.serialize(
      object.startTime,
      specifiedType: const FullType(DateTime),
    );
    yield r'trace_id';
    yield serializers.serialize(
      object.traceId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TraceDetailResponse object, {
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
    required TraceDetailResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'duration_ms':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.durationMs = valueDes;
          break;
        case r'persona_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.personaId = valueDes;
          break;
        case r'spans':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(SpanEntryResponse)]),
          ) as BuiltList<SpanEntryResponse>;
          result.spans.replace(valueDes);
          break;
        case r'start_time':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.startTime = valueDes;
          break;
        case r'trace_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.traceId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TraceDetailResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TraceDetailResponseBuilder();
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
