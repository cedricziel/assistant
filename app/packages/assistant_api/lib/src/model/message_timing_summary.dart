//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'message_timing_summary.g.dart';

/// How long a reply took, as sent on the `done` frame and in history.  Absent figures are omitted, not `null`.
///
/// Properties:
/// * [decodeMs] - Time the model spent generating tokens, in milliseconds.
/// * [outputTokens] - Output tokens summed over every model call in the turn.
/// * [timeToFirstTokenMs] - Turn start to the first visible answer token, in milliseconds.
/// * [totalMs] - Turn start to just before the final answer was saved, in milliseconds.
@BuiltValue()
abstract class MessageTimingSummary
    implements Built<MessageTimingSummary, MessageTimingSummaryBuilder> {
  /// Time the model spent generating tokens, in milliseconds.
  @BuiltValueField(wireName: r'decode_ms')
  int? get decodeMs;

  /// Output tokens summed over every model call in the turn.
  @BuiltValueField(wireName: r'output_tokens')
  int? get outputTokens;

  /// Turn start to the first visible answer token, in milliseconds.
  @BuiltValueField(wireName: r'time_to_first_token_ms')
  int? get timeToFirstTokenMs;

  /// Turn start to just before the final answer was saved, in milliseconds.
  @BuiltValueField(wireName: r'total_ms')
  int get totalMs;

  MessageTimingSummary._();

  factory MessageTimingSummary([void updates(MessageTimingSummaryBuilder b)]) =
      _$MessageTimingSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(MessageTimingSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<MessageTimingSummary> get serializer =>
      _$MessageTimingSummarySerializer();
}

class _$MessageTimingSummarySerializer
    implements PrimitiveSerializer<MessageTimingSummary> {
  @override
  final Iterable<Type> types = const [
    MessageTimingSummary,
    _$MessageTimingSummary
  ];

  @override
  final String wireName = r'MessageTimingSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    MessageTimingSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.decodeMs != null) {
      yield r'decode_ms';
      yield serializers.serialize(
        object.decodeMs,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.outputTokens != null) {
      yield r'output_tokens';
      yield serializers.serialize(
        object.outputTokens,
        specifiedType: const FullType.nullable(int),
      );
    }
    if (object.timeToFirstTokenMs != null) {
      yield r'time_to_first_token_ms';
      yield serializers.serialize(
        object.timeToFirstTokenMs,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'total_ms';
    yield serializers.serialize(
      object.totalMs,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    MessageTimingSummary object, {
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
    required MessageTimingSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'decode_ms':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.decodeMs = valueDes;
          break;
        case r'output_tokens':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.outputTokens = valueDes;
          break;
        case r'time_to_first_token_ms':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.timeToFirstTokenMs = valueDes;
          break;
        case r'total_ms':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalMs = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  MessageTimingSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = MessageTimingSummaryBuilder();
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
