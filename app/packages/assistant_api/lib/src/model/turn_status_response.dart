//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/turn_state.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'turn_status_response.g.dart';

/// Response body for `GET .../turns/{turn_id}/status`.
///
/// Properties:
/// * [conversationId]
/// * [lastEventAt] - Wall-clock timestamp of the most recent SSE event recorded for the turn, in RFC 3339 format. `null` when [`state`] is `unknown`.
/// * [lastEventKind] - Kind of the most recent SSE event (`run_started`, `token`, `status`, `thinking`, `tool_result`, `agent_error`, `done`, etc.). `null` when [`state`] is `unknown`.
/// * [state]
/// * [turnId]
@BuiltValue()
abstract class TurnStatusResponse
    implements Built<TurnStatusResponse, TurnStatusResponseBuilder> {
  @BuiltValueField(wireName: r'conversation_id')
  String get conversationId;

  /// Wall-clock timestamp of the most recent SSE event recorded for the turn, in RFC 3339 format. `null` when [`state`] is `unknown`.
  @BuiltValueField(wireName: r'last_event_at')
  String? get lastEventAt;

  /// Kind of the most recent SSE event (`run_started`, `token`, `status`, `thinking`, `tool_result`, `agent_error`, `done`, etc.). `null` when [`state`] is `unknown`.
  @BuiltValueField(wireName: r'last_event_kind')
  String? get lastEventKind;

  @BuiltValueField(wireName: r'state')
  TurnState get state;
  // enum stateEnum {  running,  completed,  errored,  unknown,  };

  @BuiltValueField(wireName: r'turn_id')
  String get turnId;

  TurnStatusResponse._();

  factory TurnStatusResponse([void updates(TurnStatusResponseBuilder b)]) =
      _$TurnStatusResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TurnStatusResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TurnStatusResponse> get serializer =>
      _$TurnStatusResponseSerializer();
}

class _$TurnStatusResponseSerializer
    implements PrimitiveSerializer<TurnStatusResponse> {
  @override
  final Iterable<Type> types = const [TurnStatusResponse, _$TurnStatusResponse];

  @override
  final String wireName = r'TurnStatusResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TurnStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'conversation_id';
    yield serializers.serialize(
      object.conversationId,
      specifiedType: const FullType(String),
    );
    if (object.lastEventAt != null) {
      yield r'last_event_at';
      yield serializers.serialize(
        object.lastEventAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.lastEventKind != null) {
      yield r'last_event_kind';
      yield serializers.serialize(
        object.lastEventKind,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'state';
    yield serializers.serialize(
      object.state,
      specifiedType: const FullType(TurnState),
    );
    yield r'turn_id';
    yield serializers.serialize(
      object.turnId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TurnStatusResponse object, {
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
    required TurnStatusResponseBuilder result,
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
        case r'last_event_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.lastEventAt = valueDes;
          break;
        case r'last_event_kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.lastEventKind = valueDes;
          break;
        case r'state':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(TurnState),
          ) as TurnState;
          result.state = valueDes;
          break;
        case r'turn_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.turnId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TurnStatusResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TurnStatusResponseBuilder();
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
