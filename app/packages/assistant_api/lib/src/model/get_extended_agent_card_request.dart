//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'get_extended_agent_card_request.g.dart';

/// Request for the `GetExtendedAgentCard` method.
///
/// Properties:
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class GetExtendedAgentCardRequest implements Built<GetExtendedAgentCardRequest, GetExtendedAgentCardRequestBuilder> {
  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  GetExtendedAgentCardRequest._();

  factory GetExtendedAgentCardRequest([void updates(GetExtendedAgentCardRequestBuilder b)]) = _$GetExtendedAgentCardRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GetExtendedAgentCardRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GetExtendedAgentCardRequest> get serializer => _$GetExtendedAgentCardRequestSerializer();
}

class _$GetExtendedAgentCardRequestSerializer implements PrimitiveSerializer<GetExtendedAgentCardRequest> {
  @override
  final Iterable<Type> types = const [GetExtendedAgentCardRequest, _$GetExtendedAgentCardRequest];

  @override
  final String wireName = r'GetExtendedAgentCardRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GetExtendedAgentCardRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.tenant != null) {
      yield r'tenant';
      yield serializers.serialize(
        object.tenant,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    GetExtendedAgentCardRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GetExtendedAgentCardRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'tenant':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.tenant = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  GetExtendedAgentCardRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GetExtendedAgentCardRequestBuilder();
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

