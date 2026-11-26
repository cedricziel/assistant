//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'surface_summary.g.dart';

/// One surface of a conversation, as the client needs to render it again.
///
/// Properties:
/// * [actionMessageId] - The user message that carried the submitting action.
/// * [anchorCallIndex] - Index of the creating call inside that message's tool calls.
/// * [anchorMessageId] - The tool-call message the surface hangs off.
/// * [answerable] - Whether the user can still answer the surface: it is `active` and no user message follows its anchor on the returned path.
/// * [createdAt]
/// * [fallbackText]
/// * [messages] - A single synthesized A2UI `createSurface` holding the current components and data model.
/// * [status] - `\"active\"` or `\"submitted\"`.
/// * [submittedAction] - The action that submitted the surface.
/// * [surfaceId]
/// * [title]
/// * [toolCallId] - Provider id of the creating tool call, when the provider supplied one.
/// * [turn]
@BuiltValue()
abstract class SurfaceSummary
    implements Built<SurfaceSummary, SurfaceSummaryBuilder> {
  /// The user message that carried the submitting action.
  @BuiltValueField(wireName: r'action_message_id')
  String? get actionMessageId;

  /// Index of the creating call inside that message's tool calls.
  @BuiltValueField(wireName: r'anchor_call_index')
  int get anchorCallIndex;

  /// The tool-call message the surface hangs off.
  @BuiltValueField(wireName: r'anchor_message_id')
  String get anchorMessageId;

  /// Whether the user can still answer the surface: it is `active` and no user message follows its anchor on the returned path.
  @BuiltValueField(wireName: r'answerable')
  bool get answerable;

  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'fallback_text')
  String get fallbackText;

  /// A single synthesized A2UI `createSurface` holding the current components and data model.
  @BuiltValueField(wireName: r'messages')
  BuiltList<JsonObject> get messages;

  /// `\"active\"` or `\"submitted\"`.
  @BuiltValueField(wireName: r'status')
  String get status;

  /// The action that submitted the surface.
  @BuiltValueField(wireName: r'submitted_action')
  JsonObject? get submittedAction;

  @BuiltValueField(wireName: r'surface_id')
  String get surfaceId;

  @BuiltValueField(wireName: r'title')
  String get title;

  /// Provider id of the creating tool call, when the provider supplied one.
  @BuiltValueField(wireName: r'tool_call_id')
  String? get toolCallId;

  @BuiltValueField(wireName: r'turn')
  int get turn;

  SurfaceSummary._();

  factory SurfaceSummary([void updates(SurfaceSummaryBuilder b)]) =
      _$SurfaceSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SurfaceSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SurfaceSummary> get serializer =>
      _$SurfaceSummarySerializer();
}

class _$SurfaceSummarySerializer
    implements PrimitiveSerializer<SurfaceSummary> {
  @override
  final Iterable<Type> types = const [SurfaceSummary, _$SurfaceSummary];

  @override
  final String wireName = r'SurfaceSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SurfaceSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.actionMessageId != null) {
      yield r'action_message_id';
      yield serializers.serialize(
        object.actionMessageId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'anchor_call_index';
    yield serializers.serialize(
      object.anchorCallIndex,
      specifiedType: const FullType(int),
    );
    yield r'anchor_message_id';
    yield serializers.serialize(
      object.anchorMessageId,
      specifiedType: const FullType(String),
    );
    yield r'answerable';
    yield serializers.serialize(
      object.answerable,
      specifiedType: const FullType(bool),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'fallback_text';
    yield serializers.serialize(
      object.fallbackText,
      specifiedType: const FullType(String),
    );
    yield r'messages';
    yield serializers.serialize(
      object.messages,
      specifiedType: const FullType(BuiltList, [FullType(JsonObject)]),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    if (object.submittedAction != null) {
      yield r'submitted_action';
      yield serializers.serialize(
        object.submittedAction,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'surface_id';
    yield serializers.serialize(
      object.surfaceId,
      specifiedType: const FullType(String),
    );
    yield r'title';
    yield serializers.serialize(
      object.title,
      specifiedType: const FullType(String),
    );
    if (object.toolCallId != null) {
      yield r'tool_call_id';
      yield serializers.serialize(
        object.toolCallId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'turn';
    yield serializers.serialize(
      object.turn,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SurfaceSummary object, {
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
    required SurfaceSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'action_message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.actionMessageId = valueDes;
          break;
        case r'anchor_call_index':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.anchorCallIndex = valueDes;
          break;
        case r'anchor_message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.anchorMessageId = valueDes;
          break;
        case r'answerable':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.answerable = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'fallback_text':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fallbackText = valueDes;
          break;
        case r'messages':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(JsonObject)]),
          ) as BuiltList<JsonObject>;
          result.messages.replace(valueDes);
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'submitted_action':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.submittedAction = valueDes;
          break;
        case r'surface_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.surfaceId = valueDes;
          break;
        case r'title':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.title = valueDes;
          break;
        case r'tool_call_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.toolCallId = valueDes;
          break;
        case r'turn':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.turn = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SurfaceSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SurfaceSummaryBuilder();
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
