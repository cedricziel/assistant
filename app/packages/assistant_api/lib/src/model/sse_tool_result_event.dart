//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_tool_result_event.g.dart';

/// SSE `tool_result` event — result of a tool invocation.
///
/// Properties:
/// * [arguments] - The arguments passed to the tool (JSON string).
/// * [result] - The tool's output or error message.
/// * [status] - Whether the tool call succeeded or failed.
/// * [toolName] - Name of the tool that was called.
@BuiltValue()
abstract class SseToolResultEvent
    implements Built<SseToolResultEvent, SseToolResultEventBuilder> {
  /// The arguments passed to the tool (JSON string).
  @BuiltValueField(wireName: r'arguments')
  String? get arguments;

  /// The tool's output or error message.
  @BuiltValueField(wireName: r'result')
  String? get result;

  /// Whether the tool call succeeded or failed.
  @BuiltValueField(wireName: r'status')
  String get status;

  /// Name of the tool that was called.
  @BuiltValueField(wireName: r'tool_name')
  String get toolName;

  SseToolResultEvent._();

  factory SseToolResultEvent([void updates(SseToolResultEventBuilder b)]) =
      _$SseToolResultEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseToolResultEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseToolResultEvent> get serializer =>
      _$SseToolResultEventSerializer();
}

class _$SseToolResultEventSerializer
    implements PrimitiveSerializer<SseToolResultEvent> {
  @override
  final Iterable<Type> types = const [SseToolResultEvent, _$SseToolResultEvent];

  @override
  final String wireName = r'SseToolResultEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseToolResultEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.arguments != null) {
      yield r'arguments';
      yield serializers.serialize(
        object.arguments,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.result != null) {
      yield r'result';
      yield serializers.serialize(
        object.result,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'tool_name';
    yield serializers.serialize(
      object.toolName,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SseToolResultEvent object, {
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
    required SseToolResultEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'arguments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.arguments = valueDes;
          break;
        case r'result':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.result = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'tool_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.toolName = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseToolResultEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseToolResultEventBuilder();
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
