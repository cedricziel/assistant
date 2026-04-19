//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_summary.g.dart';

/// Workflow list item.
///
/// Properties:
/// * [actionCount] 
/// * [active] 
/// * [createdAt] 
/// * [description] 
/// * [edgeCount] 
/// * [id] - Workflow UUID.
/// * [maxSteps] 
/// * [maxVisitsPerNode] 
/// * [name] 
/// * [nodeCount] 
/// * [triggerCount] 
/// * [updatedAt] 
@BuiltValue()
abstract class WorkflowSummary implements Built<WorkflowSummary, WorkflowSummaryBuilder> {
  @BuiltValueField(wireName: r'action_count')
  int get actionCount;

  @BuiltValueField(wireName: r'active')
  bool get active;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'edge_count')
  int get edgeCount;

  /// Workflow UUID.
  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'max_steps')
  int get maxSteps;

  @BuiltValueField(wireName: r'max_visits_per_node')
  int get maxVisitsPerNode;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'node_count')
  int get nodeCount;

  @BuiltValueField(wireName: r'trigger_count')
  int get triggerCount;

  @BuiltValueField(wireName: r'updated_at')
  DateTime get updatedAt;

  WorkflowSummary._();

  factory WorkflowSummary([void updates(WorkflowSummaryBuilder b)]) = _$WorkflowSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowSummary> get serializer => _$WorkflowSummarySerializer();
}

class _$WorkflowSummarySerializer implements PrimitiveSerializer<WorkflowSummary> {
  @override
  final Iterable<Type> types = const [WorkflowSummary, _$WorkflowSummary];

  @override
  final String wireName = r'WorkflowSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'action_count';
    yield serializers.serialize(
      object.actionCount,
      specifiedType: const FullType(int),
    );
    yield r'active';
    yield serializers.serialize(
      object.active,
      specifiedType: const FullType(bool),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'edge_count';
    yield serializers.serialize(
      object.edgeCount,
      specifiedType: const FullType(int),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'max_steps';
    yield serializers.serialize(
      object.maxSteps,
      specifiedType: const FullType(int),
    );
    yield r'max_visits_per_node';
    yield serializers.serialize(
      object.maxVisitsPerNode,
      specifiedType: const FullType(int),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'node_count';
    yield serializers.serialize(
      object.nodeCount,
      specifiedType: const FullType(int),
    );
    yield r'trigger_count';
    yield serializers.serialize(
      object.triggerCount,
      specifiedType: const FullType(int),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(DateTime),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkflowSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required WorkflowSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'action_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.actionCount = valueDes;
          break;
        case r'active':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.active = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'edge_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.edgeCount = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'max_steps':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.maxSteps = valueDes;
          break;
        case r'max_visits_per_node':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.maxVisitsPerNode = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'node_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.nodeCount = valueDes;
          break;
        case r'trigger_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.triggerCount = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  WorkflowSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowSummaryBuilder();
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

