//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tool_call_summary.g.dart';

/// Summary of a single tool invocation within an assistant message.
///
/// Properties:
/// * [arguments]
/// * [data] - Structured output of the tool, a JSON object with a `kind` tag. Absent when the tool returned text only, the data exceeded the size limit, or the call predates structured data. Each `kind` is defined by a JSON Schema in `schemas/tool-results/`.
/// * [name] - The tool that was called.
/// * [result] - The tool's output, truncated to a reasonable display length.
/// * [status] - `\"ok\"`, `\"error\"`, or `\"denied\"`.
/// * [toolCallId] - Provider-assigned id of the call, when the provider supplied one.
@BuiltValue()
abstract class ToolCallSummary
    implements Built<ToolCallSummary, ToolCallSummaryBuilder> {
  @BuiltValueField(wireName: r'arguments')
  JsonObject? get arguments;

  /// Structured output of the tool, a JSON object with a `kind` tag. Absent when the tool returned text only, the data exceeded the size limit, or the call predates structured data. Each `kind` is defined by a JSON Schema in `schemas/tool-results/`.
  @BuiltValueField(wireName: r'data')
  JsonObject? get data;

  /// The tool that was called.
  @BuiltValueField(wireName: r'name')
  String get name;

  /// The tool's output, truncated to a reasonable display length.
  @BuiltValueField(wireName: r'result')
  String? get result;

  /// `\"ok\"`, `\"error\"`, or `\"denied\"`.
  @BuiltValueField(wireName: r'status')
  String get status;

  /// Provider-assigned id of the call, when the provider supplied one.
  @BuiltValueField(wireName: r'tool_call_id')
  String? get toolCallId;

  ToolCallSummary._();

  factory ToolCallSummary([void updates(ToolCallSummaryBuilder b)]) =
      _$ToolCallSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ToolCallSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ToolCallSummary> get serializer =>
      _$ToolCallSummarySerializer();
}

class _$ToolCallSummarySerializer
    implements PrimitiveSerializer<ToolCallSummary> {
  @override
  final Iterable<Type> types = const [ToolCallSummary, _$ToolCallSummary];

  @override
  final String wireName = r'ToolCallSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ToolCallSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.arguments != null) {
      yield r'arguments';
      yield serializers.serialize(
        object.arguments,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    if (object.data != null) {
      yield r'data';
      yield serializers.serialize(
        object.data,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
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
    if (object.toolCallId != null) {
      yield r'tool_call_id';
      yield serializers.serialize(
        object.toolCallId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ToolCallSummary object, {
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
    required ToolCallSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'arguments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.arguments = valueDes;
          break;
        case r'data':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.data = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
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
        case r'tool_call_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.toolCallId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ToolCallSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ToolCallSummaryBuilder();
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
