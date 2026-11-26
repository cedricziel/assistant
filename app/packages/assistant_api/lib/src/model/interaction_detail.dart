//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/interaction_status_view.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'interaction_detail.g.dart';

/// A turn interaction: the request the turn put to its owner and, once it ended, how.
///
/// Properties:
/// * [conversationId]
/// * [createdAt]
/// * [expiresAt]
/// * [interactionId]
/// * [kind] - `tool_approval`, `question`, or a kind a newer server added.
/// * [rendersCard] - Whether the kind renders a card in the timeline.
/// * [request] - The kind-specific request (for `tool_approval`: `tool_name`, `arguments`, `arguments_truncated`, `reason`, `options`, `suggested_rule`; for `question`: `questions`).
/// * [resolvedAt]
/// * [response] - The owner's answer. Present once `status` is `resolved`.
/// * [ruleId] - Id of the permission rule an \"always\" answer saved.
/// * [runId] - The run (turn) that asked.
/// * [status]
/// * [toolCallId] - Provider id of the tool call this interaction belongs to, if any.
@BuiltValue()
abstract class InteractionDetail
    implements Built<InteractionDetail, InteractionDetailBuilder> {
  @BuiltValueField(wireName: r'conversation_id')
  String get conversationId;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'expires_at')
  DateTime get expiresAt;

  @BuiltValueField(wireName: r'interaction_id')
  String get interactionId;

  /// `tool_approval`, `question`, or a kind a newer server added.
  @BuiltValueField(wireName: r'kind')
  String get kind;

  /// Whether the kind renders a card in the timeline.
  @BuiltValueField(wireName: r'renders_card')
  bool get rendersCard;

  /// The kind-specific request (for `tool_approval`: `tool_name`, `arguments`, `arguments_truncated`, `reason`, `options`, `suggested_rule`; for `question`: `questions`).
  @BuiltValueField(wireName: r'request')
  JsonObject get request;

  @BuiltValueField(wireName: r'resolved_at')
  DateTime? get resolvedAt;

  /// The owner's answer. Present once `status` is `resolved`.
  @BuiltValueField(wireName: r'response')
  JsonObject? get response;

  /// Id of the permission rule an \"always\" answer saved.
  @BuiltValueField(wireName: r'rule_id')
  String? get ruleId;

  /// The run (turn) that asked.
  @BuiltValueField(wireName: r'run_id')
  String get runId;

  @BuiltValueField(wireName: r'status')
  InteractionStatusView get status;
  // enum statusEnum {  pending,  resolved,  cancelled,  expired,  };

  /// Provider id of the tool call this interaction belongs to, if any.
  @BuiltValueField(wireName: r'tool_call_id')
  String? get toolCallId;

  InteractionDetail._();

  factory InteractionDetail([void updates(InteractionDetailBuilder b)]) =
      _$InteractionDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(InteractionDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<InteractionDetail> get serializer =>
      _$InteractionDetailSerializer();
}

class _$InteractionDetailSerializer
    implements PrimitiveSerializer<InteractionDetail> {
  @override
  final Iterable<Type> types = const [InteractionDetail, _$InteractionDetail];

  @override
  final String wireName = r'InteractionDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    InteractionDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'conversation_id';
    yield serializers.serialize(
      object.conversationId,
      specifiedType: const FullType(String),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'expires_at';
    yield serializers.serialize(
      object.expiresAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'interaction_id';
    yield serializers.serialize(
      object.interactionId,
      specifiedType: const FullType(String),
    );
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(String),
    );
    yield r'renders_card';
    yield serializers.serialize(
      object.rendersCard,
      specifiedType: const FullType(bool),
    );
    yield r'request';
    yield serializers.serialize(
      object.request,
      specifiedType: const FullType(JsonObject),
    );
    if (object.resolvedAt != null) {
      yield r'resolved_at';
      yield serializers.serialize(
        object.resolvedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
    if (object.response != null) {
      yield r'response';
      yield serializers.serialize(
        object.response,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    if (object.ruleId != null) {
      yield r'rule_id';
      yield serializers.serialize(
        object.ruleId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'run_id';
    yield serializers.serialize(
      object.runId,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(InteractionStatusView),
    );
    if (object.toolCallId != null) {
      yield r'tool_call_id';
      yield serializers.serialize(
        object.toolCallId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    InteractionDetail object, {
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
    required InteractionDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'conversation_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.conversationId = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.expiresAt = valueDes;
          break;
        case r'interaction_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.interactionId = valueDes;
          break;
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.kind = valueDes;
          break;
        case r'renders_card':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.rendersCard = valueDes;
          break;
        case r'request':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(JsonObject),
          ) as JsonObject;
          result.request = valueDes;
          break;
        case r'resolved_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.resolvedAt = valueDes;
          break;
        case r'response':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.response = valueDes;
          break;
        case r'rule_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.ruleId = valueDes;
          break;
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
            specifiedType: const FullType(InteractionStatusView),
          ) as InteractionStatusView;
          result.status = valueDes;
          break;
        case r'tool_call_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.toolCallId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  InteractionDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = InteractionDetailBuilder();
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
