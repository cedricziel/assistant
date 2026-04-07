//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/workflow_run_step.dart';
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/workflow_run_summary.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_run_detail.g.dart';

/// Workflow run detail including all steps.
///
/// Properties:
/// * [run] 
/// * [steps] 
@BuiltValue()
abstract class WorkflowRunDetail implements Built<WorkflowRunDetail, WorkflowRunDetailBuilder> {
  @BuiltValueField(wireName: r'run')
  WorkflowRunSummary get run;

  @BuiltValueField(wireName: r'steps')
  BuiltList<WorkflowRunStep> get steps;

  WorkflowRunDetail._();

  factory WorkflowRunDetail([void updates(WorkflowRunDetailBuilder b)]) = _$WorkflowRunDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowRunDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowRunDetail> get serializer => _$WorkflowRunDetailSerializer();
}

class _$WorkflowRunDetailSerializer implements PrimitiveSerializer<WorkflowRunDetail> {
  @override
  final Iterable<Type> types = const [WorkflowRunDetail, _$WorkflowRunDetail];

  @override
  final String wireName = r'WorkflowRunDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowRunDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'run';
    yield serializers.serialize(
      object.run,
      specifiedType: const FullType(WorkflowRunSummary),
    );
    yield r'steps';
    yield serializers.serialize(
      object.steps,
      specifiedType: const FullType(BuiltList, [FullType(WorkflowRunStep)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkflowRunDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required WorkflowRunDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'run':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(WorkflowRunSummary),
          ) as WorkflowRunSummary;
          result.run.replace(valueDes);
          break;
        case r'steps':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(WorkflowRunStep)]),
          ) as BuiltList<WorkflowRunStep>;
          result.steps.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  WorkflowRunDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowRunDetailBuilder();
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

