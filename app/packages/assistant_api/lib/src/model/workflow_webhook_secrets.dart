//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'workflow_webhook_secrets.g.dart';

/// Webhook URL and token for a workflow trigger.
///
/// Properties:
/// * [webhookToken] 
/// * [webhookUrl] 
@BuiltValue()
abstract class WorkflowWebhookSecrets implements Built<WorkflowWebhookSecrets, WorkflowWebhookSecretsBuilder> {
  @BuiltValueField(wireName: r'webhook_token')
  String get webhookToken;

  @BuiltValueField(wireName: r'webhook_url')
  String get webhookUrl;

  WorkflowWebhookSecrets._();

  factory WorkflowWebhookSecrets([void updates(WorkflowWebhookSecretsBuilder b)]) = _$WorkflowWebhookSecrets;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WorkflowWebhookSecretsBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WorkflowWebhookSecrets> get serializer => _$WorkflowWebhookSecretsSerializer();
}

class _$WorkflowWebhookSecretsSerializer implements PrimitiveSerializer<WorkflowWebhookSecrets> {
  @override
  final Iterable<Type> types = const [WorkflowWebhookSecrets, _$WorkflowWebhookSecrets];

  @override
  final String wireName = r'WorkflowWebhookSecrets';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WorkflowWebhookSecrets object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'webhook_token';
    yield serializers.serialize(
      object.webhookToken,
      specifiedType: const FullType(String),
    );
    yield r'webhook_url';
    yield serializers.serialize(
      object.webhookUrl,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WorkflowWebhookSecrets object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required WorkflowWebhookSecretsBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'webhook_token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.webhookToken = valueDes;
          break;
        case r'webhook_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.webhookUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  WorkflowWebhookSecrets deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WorkflowWebhookSecretsBuilder();
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

