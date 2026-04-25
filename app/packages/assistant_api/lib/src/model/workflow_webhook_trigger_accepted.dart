//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_webhook_trigger_accepted.g.dart';

/// Accepted response for a public webhook trigger.
///
/// Properties:
/// * [runId]
/// * [status]
/// * [workflowId]
@BuiltValue()
abstract class WorkflowWebhookTriggerAccepted
    implements
        Built<WorkflowWebhookTriggerAccepted,
            WorkflowWebhookTriggerAcceptedBuilder> {
  @BuiltValueField(wireName: r'run_id')
  String get runId;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'workflow_id')
  String get workflowId;

  WorkflowWebhookTriggerAccepted._();

  factory WorkflowWebhookTriggerAccepted(
          [void updates(WorkflowWebhookTriggerAcceptedBuilder b)]) =
      _$WorkflowWebhookTriggerAccepted;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowWebhookTriggerAcceptedBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowWebhookTriggerAccepted> get serializer =>
      _$WorkflowWebhookTriggerAcceptedSerializer();
}

class _$WorkflowWebhookTriggerAcceptedSerializer
    implements PrimitiveSerializer<WorkflowWebhookTriggerAccepted> {
  @override
  final Iterable<Type> types = const [
    WorkflowWebhookTriggerAccepted,
    _$WorkflowWebhookTriggerAccepted
  ];

  @override
  final String wireName = r'WorkflowWebhookTriggerAccepted';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowWebhookTriggerAccepted object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'run_id';
    yield serializers.serialize(
      object.runId,
      specifiedType: const FullType(String),
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
    WorkflowWebhookTriggerAccepted object, {
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
    required WorkflowWebhookTriggerAcceptedBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'run_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.runId = valueDes;
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
  WorkflowWebhookTriggerAccepted deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowWebhookTriggerAcceptedBuilder();
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
