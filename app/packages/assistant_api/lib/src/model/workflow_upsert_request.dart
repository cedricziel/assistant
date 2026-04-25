//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_upsert_request.g.dart';

/// Body for `POST /api/workflows` and `PUT /api/workflows/{id}`.
///
/// Properties:
/// * [active]
/// * [description]
/// * [graph]
/// * [name]
@BuiltValue()
abstract class WorkflowUpsertRequest
    implements Built<WorkflowUpsertRequest, WorkflowUpsertRequestBuilder> {
  @BuiltValueField(wireName: r'active')
  bool? get active;

  @BuiltValueField(wireName: r'description')
  String? get description;

  @BuiltValueField(wireName: r'graph')
  JsonObject? get graph;

  @BuiltValueField(wireName: r'name')
  String get name;

  WorkflowUpsertRequest._();

  factory WorkflowUpsertRequest(
      [void updates(WorkflowUpsertRequestBuilder b)]) = _$WorkflowUpsertRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowUpsertRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowUpsertRequest> get serializer =>
      _$WorkflowUpsertRequestSerializer();
}

class _$WorkflowUpsertRequestSerializer
    implements PrimitiveSerializer<WorkflowUpsertRequest> {
  @override
  final Iterable<Type> types = const [
    WorkflowUpsertRequest,
    _$WorkflowUpsertRequest
  ];

  @override
  final String wireName = r'WorkflowUpsertRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowUpsertRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.active != null) {
      yield r'active';
      yield serializers.serialize(
        object.active,
        specifiedType: const FullType(bool),
      );
    }
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType(String),
      );
    }
    yield r'graph';
    yield object.graph == null
        ? null
        : serializers.serialize(
            object.graph,
            specifiedType: const FullType.nullable(JsonObject),
          );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkflowUpsertRequest object, {
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
    required WorkflowUpsertRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'active':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.active = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'graph':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.graph = valueDes;
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
  WorkflowUpsertRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowUpsertRequestBuilder();
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
