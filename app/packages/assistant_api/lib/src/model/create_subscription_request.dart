//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_subscription_request.g.dart';

/// CreateSubscriptionRequest
///
/// Properties:
/// * [catalogItemId]
@BuiltValue()
abstract class CreateSubscriptionRequest
    implements
        Built<CreateSubscriptionRequest, CreateSubscriptionRequestBuilder> {
  @BuiltValueField(wireName: r'catalog_item_id')
  String get catalogItemId;

  CreateSubscriptionRequest._();

  factory CreateSubscriptionRequest(
          [void updates(CreateSubscriptionRequestBuilder b)]) =
      _$CreateSubscriptionRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateSubscriptionRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateSubscriptionRequest> get serializer =>
      _$CreateSubscriptionRequestSerializer();
}

class _$CreateSubscriptionRequestSerializer
    implements PrimitiveSerializer<CreateSubscriptionRequest> {
  @override
  final Iterable<Type> types = const [
    CreateSubscriptionRequest,
    _$CreateSubscriptionRequest
  ];

  @override
  final String wireName = r'CreateSubscriptionRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateSubscriptionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'catalog_item_id';
    yield serializers.serialize(
      object.catalogItemId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateSubscriptionRequest object, {
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
    required CreateSubscriptionRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'catalog_item_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.catalogItemId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateSubscriptionRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateSubscriptionRequestBuilder();
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
