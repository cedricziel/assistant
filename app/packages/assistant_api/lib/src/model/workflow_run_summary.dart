//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_run_summary.g.dart';

/// Single workflow run.
///
/// Properties:
/// * [errorMessage] 
/// * [finishedAt] 
/// * [id] 
/// * [startedAt] 
/// * [status] 
/// * [triggerPayload] 
/// * [triggerType] 
/// * [workflowId] 
@BuiltValue()
abstract class WorkflowRunSummary implements Built<WorkflowRunSummary, WorkflowRunSummaryBuilder> {
  @BuiltValueField(wireName: r'error_message')
  String? get errorMessage;

  @BuiltValueField(wireName: r'finished_at')
  DateTime? get finishedAt;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'started_at')
  DateTime get startedAt;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'trigger_payload')
  JsonObject? get triggerPayload;

  @BuiltValueField(wireName: r'trigger_type')
  String get triggerType;

  @BuiltValueField(wireName: r'workflow_id')
  String get workflowId;

  WorkflowRunSummary._();

  factory WorkflowRunSummary([void updates(WorkflowRunSummaryBuilder b)]) = _$WorkflowRunSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowRunSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowRunSummary> get serializer => _$WorkflowRunSummarySerializer();
}

class _$WorkflowRunSummarySerializer implements PrimitiveSerializer<WorkflowRunSummary> {
  @override
  final Iterable<Type> types = const [WorkflowRunSummary, _$WorkflowRunSummary];

  @override
  final String wireName = r'WorkflowRunSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowRunSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.errorMessage != null) {
      yield r'error_message';
      yield serializers.serialize(
        object.errorMessage,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.finishedAt != null) {
      yield r'finished_at';
      yield serializers.serialize(
        object.finishedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
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
    yield r'trigger_payload';
    yield object.triggerPayload == null ? null : serializers.serialize(
      object.triggerPayload,
      specifiedType: const FullType.nullable(JsonObject),
    );
    yield r'trigger_type';
    yield serializers.serialize(
      object.triggerType,
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
    WorkflowRunSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required WorkflowRunSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'error_message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.errorMessage = valueDes;
          break;
        case r'finished_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.finishedAt = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
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
        case r'trigger_payload':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.triggerPayload = valueDes;
          break;
        case r'trigger_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.triggerType = valueDes;
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
  WorkflowRunSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowRunSummaryBuilder();
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

