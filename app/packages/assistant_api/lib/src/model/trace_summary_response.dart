//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'trace_summary_response.g.dart';

/// A trace summary row.
///
/// Properties:
/// * [conversationId] 
/// * [durationMs] 
/// * [personaId] 
/// * [skillName] 
/// * [startTime] 
/// * [status] 
/// * [traceId] 
@BuiltValue()
abstract class TraceSummaryResponse implements Built<TraceSummaryResponse, TraceSummaryResponseBuilder> {
  @BuiltValueField(wireName: r'conversation_id')
  String? get conversationId;

  @BuiltValueField(wireName: r'duration_ms')
  int get durationMs;

  @BuiltValueField(wireName: r'persona_id')
  String get personaId;

  @BuiltValueField(wireName: r'skill_name')
  String? get skillName;

  @BuiltValueField(wireName: r'start_time')
  DateTime get startTime;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'trace_id')
  String get traceId;

  TraceSummaryResponse._();

  factory TraceSummaryResponse([void updates(TraceSummaryResponseBuilder b)]) = _$TraceSummaryResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TraceSummaryResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TraceSummaryResponse> get serializer => _$TraceSummaryResponseSerializer();
}

class _$TraceSummaryResponseSerializer implements PrimitiveSerializer<TraceSummaryResponse> {
  @override
  final Iterable<Type> types = const [TraceSummaryResponse, _$TraceSummaryResponse];

  @override
  final String wireName = r'TraceSummaryResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TraceSummaryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.conversationId != null) {
      yield r'conversation_id';
      yield serializers.serialize(
        object.conversationId,
        specifiedType: const FullType.nullable(String),
      );
    }
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
    if (object.skillName != null) {
      yield r'skill_name';
      yield serializers.serialize(
        object.skillName,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'start_time';
    yield serializers.serialize(
      object.startTime,
      specifiedType: const FullType(DateTime),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
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
    TraceSummaryResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TraceSummaryResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'conversation_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.conversationId = valueDes;
          break;
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
        case r'skill_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.skillName = valueDes;
          break;
        case r'start_time':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.startTime = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
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
  TraceSummaryResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TraceSummaryResponseBuilder();
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

