//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'subscribe_to_task_request.g.dart';

/// Request for the `SubscribeToTask` method.
///
/// Properties:
/// * [id] - The resource ID of the task to subscribe to.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class SubscribeToTaskRequest implements Built<SubscribeToTaskRequest, SubscribeToTaskRequestBuilder> {
  /// The resource ID of the task to subscribe to.
  @BuiltValueField(wireName: r'id')
  String get id;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  SubscribeToTaskRequest._();

  factory SubscribeToTaskRequest([void updates(SubscribeToTaskRequestBuilder b)]) = _$SubscribeToTaskRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SubscribeToTaskRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SubscribeToTaskRequest> get serializer => _$SubscribeToTaskRequestSerializer();
}

class _$SubscribeToTaskRequestSerializer implements PrimitiveSerializer<SubscribeToTaskRequest> {
  @override
  final Iterable<Type> types = const [SubscribeToTaskRequest, _$SubscribeToTaskRequest];

  @override
  final String wireName = r'SubscribeToTaskRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SubscribeToTaskRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
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
    SubscribeToTaskRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SubscribeToTaskRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
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
  SubscribeToTaskRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SubscribeToTaskRequestBuilder();
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

