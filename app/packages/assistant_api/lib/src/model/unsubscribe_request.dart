//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'unsubscribe_request.g.dart';

/// Endpoint URL of the subscription to remove.
///
/// Properties:
/// * [endpoint] - The push endpoint URL to remove.
@BuiltValue()
abstract class UnsubscribeRequest
    implements Built<UnsubscribeRequest, UnsubscribeRequestBuilder> {
  /// The push endpoint URL to remove.
  @BuiltValueField(wireName: r'endpoint')
  String get endpoint;

  UnsubscribeRequest._();

  factory UnsubscribeRequest([void updates(UnsubscribeRequestBuilder b)]) =
      _$UnsubscribeRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UnsubscribeRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UnsubscribeRequest> get serializer =>
      _$UnsubscribeRequestSerializer();
}

class _$UnsubscribeRequestSerializer
    implements PrimitiveSerializer<UnsubscribeRequest> {
  @override
  final Iterable<Type> types = const [UnsubscribeRequest, _$UnsubscribeRequest];

  @override
  final String wireName = r'UnsubscribeRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UnsubscribeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'endpoint';
    yield serializers.serialize(
      object.endpoint,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UnsubscribeRequest object, {
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
    required UnsubscribeRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.endpoint = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UnsubscribeRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UnsubscribeRequestBuilder();
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
