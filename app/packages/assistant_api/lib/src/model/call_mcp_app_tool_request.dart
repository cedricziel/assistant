//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'call_mcp_app_tool_request.g.dart';

/// Body of `POST .../app/tools-call`: a call the app makes to one of its server's tools.
///
/// Properties:
/// * [arguments] - The tool's arguments.
/// * [confirmed] - The user confirmed this call. Required for a tool without `readOnlyHint: true`, and for any call the permission policy leaves to a human; without it such a call is refused with `409`.
/// * [name] - The tool's name on the app's server.
@BuiltValue()
abstract class CallMcpAppToolRequest
    implements Built<CallMcpAppToolRequest, CallMcpAppToolRequestBuilder> {
  /// The tool's arguments.
  @BuiltValueField(wireName: r'arguments')
  JsonObject? get arguments;

  /// The user confirmed this call. Required for a tool without `readOnlyHint: true`, and for any call the permission policy leaves to a human; without it such a call is refused with `409`.
  @BuiltValueField(wireName: r'confirmed')
  bool? get confirmed;

  /// The tool's name on the app's server.
  @BuiltValueField(wireName: r'name')
  String get name;

  CallMcpAppToolRequest._();

  factory CallMcpAppToolRequest(
      [void updates(CallMcpAppToolRequestBuilder b)]) = _$CallMcpAppToolRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CallMcpAppToolRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CallMcpAppToolRequest> get serializer =>
      _$CallMcpAppToolRequestSerializer();
}

class _$CallMcpAppToolRequestSerializer
    implements PrimitiveSerializer<CallMcpAppToolRequest> {
  @override
  final Iterable<Type> types = const [
    CallMcpAppToolRequest,
    _$CallMcpAppToolRequest
  ];

  @override
  final String wireName = r'CallMcpAppToolRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CallMcpAppToolRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.arguments != null) {
      yield r'arguments';
      yield serializers.serialize(
        object.arguments,
        specifiedType: const FullType(JsonObject),
      );
    }
    if (object.confirmed != null) {
      yield r'confirmed';
      yield serializers.serialize(
        object.confirmed,
        specifiedType: const FullType(bool),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CallMcpAppToolRequest object, {
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
    required CallMcpAppToolRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'arguments':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(JsonObject),
          ) as JsonObject;
          result.arguments = valueDes;
          break;
        case r'confirmed':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.confirmed = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CallMcpAppToolRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CallMcpAppToolRequestBuilder();
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
