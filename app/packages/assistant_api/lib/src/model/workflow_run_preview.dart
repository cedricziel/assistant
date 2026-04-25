//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_run_preview.g.dart';

/// Response returned when a test-run is accepted.
///
/// Properties:
/// * [maxSteps]
/// * [maxVisitsPerNode]
/// * [message]
/// * [runId]
/// * [startedAt]
/// * [status]
/// * [workflowId]
@BuiltValue()
abstract class WorkflowRunPreview
    implements Built<WorkflowRunPreview, WorkflowRunPreviewBuilder> {
  @BuiltValueField(wireName: r'max_steps')
  int get maxSteps;

  @BuiltValueField(wireName: r'max_visits_per_node')
  int get maxVisitsPerNode;

  @BuiltValueField(wireName: r'message')
  String get message;

  @BuiltValueField(wireName: r'run_id')
  String get runId;

  @BuiltValueField(wireName: r'started_at')
  DateTime get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'workflow_id')
  String get workflowId;

  WorkflowRunPreview._();

  factory WorkflowRunPreview([void updates(WorkflowRunPreviewBuilder b)]) =
      _$WorkflowRunPreview;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowRunPreviewBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowRunPreview> get serializer =>
      _$WorkflowRunPreviewSerializer();
}

class _$WorkflowRunPreviewSerializer
    implements PrimitiveSerializer<WorkflowRunPreview> {
  @override
  final Iterable<Type> types = const [WorkflowRunPreview, _$WorkflowRunPreview];

  @override
  final String wireName = r'WorkflowRunPreview';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowRunPreview object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
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
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    yield r'run_id';
    yield serializers.serialize(
      object.runId,
      specifiedType: const FullType(String),
    );
    yield r'started_at';
    yield serializers.serialize(
      object.startedAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'workflow_id';
    yield serializers.serialize(
      object.workflowId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkflowRunPreview object, {
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
    required WorkflowRunPreviewBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'run_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.runId = valueDes;
          break;
        case r'started_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.startedAt = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'workflow_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.workflowId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  WorkflowRunPreview deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowRunPreviewBuilder();
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
