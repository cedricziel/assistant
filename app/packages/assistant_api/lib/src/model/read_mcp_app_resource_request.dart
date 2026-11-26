//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'read_mcp_app_resource_request.g.dart';

/// Body of `POST .../app/resources-read`.
///
/// Properties:
/// * [uri] - The resource to read from the app's server.
@BuiltValue()
abstract class ReadMcpAppResourceRequest
    implements
        Built<ReadMcpAppResourceRequest, ReadMcpAppResourceRequestBuilder> {
  /// The resource to read from the app's server.
  @BuiltValueField(wireName: r'uri')
  String get uri;

  ReadMcpAppResourceRequest._();

  factory ReadMcpAppResourceRequest(
          [void updates(ReadMcpAppResourceRequestBuilder b)]) =
      _$ReadMcpAppResourceRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ReadMcpAppResourceRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ReadMcpAppResourceRequest> get serializer =>
      _$ReadMcpAppResourceRequestSerializer();
}

class _$ReadMcpAppResourceRequestSerializer
    implements PrimitiveSerializer<ReadMcpAppResourceRequest> {
  @override
  final Iterable<Type> types = const [
    ReadMcpAppResourceRequest,
    _$ReadMcpAppResourceRequest
  ];

  @override
  final String wireName = r'ReadMcpAppResourceRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ReadMcpAppResourceRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'uri';
    yield serializers.serialize(
      object.uri,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ReadMcpAppResourceRequest object, {
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
    required ReadMcpAppResourceRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'uri':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.uri = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ReadMcpAppResourceRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ReadMcpAppResourceRequestBuilder();
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
