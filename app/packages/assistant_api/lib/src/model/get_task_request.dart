//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'get_task_request.g.dart';

/// Request for the `GetTask` method.
///
/// Properties:
/// * [historyLength] - Max number of recent messages to include in history.
/// * [id] - The resource ID of the task to retrieve.
/// * [tenant] - Optional tenant ID.
@BuiltValue()
abstract class GetTaskRequest implements Built<GetTaskRequest, GetTaskRequestBuilder> {
  /// Max number of recent messages to include in history.
  @BuiltValueField(wireName: r'historyLength')
  int? get historyLength;

  /// The resource ID of the task to retrieve.
  @BuiltValueField(wireName: r'id')
  String get id;

  /// Optional tenant ID.
  @BuiltValueField(wireName: r'tenant')
  String? get tenant;

  GetTaskRequest._();

  factory GetTaskRequest([void updates(GetTaskRequestBuilder b)]) = _$GetTaskRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GetTaskRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GetTaskRequest> get serializer => _$GetTaskRequestSerializer();
}

class _$GetTaskRequestSerializer implements PrimitiveSerializer<GetTaskRequest> {
  @override
  final Iterable<Type> types = const [GetTaskRequest, _$GetTaskRequest];

  @override
  final String wireName = r'GetTaskRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GetTaskRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.historyLength != null) {
      yield r'historyLength';
      yield serializers.serialize(
        object.historyLength,
        specifiedType: const FullType.nullable(int),
      );
    }
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
    GetTaskRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GetTaskRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'historyLength':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.historyLength = valueDes;
          break;
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
  GetTaskRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GetTaskRequestBuilder();
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

