//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/surface_action_request.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'api_send_message_request.g.dart';

/// Body for `POST /api/conversations/{id}/messages`.
///
/// Properties:
/// * [attachmentIds] - Optional attachment IDs to include with this message.
/// * [message] - The message text to send to the assistant. Empty when `surface_action` carries the turn.
/// * [surfaceAction] - Answers an active surface after its turn ended. When present, `message` must be empty; the server builds the visible text.
@BuiltValue()
abstract class ApiSendMessageRequest
    implements Built<ApiSendMessageRequest, ApiSendMessageRequestBuilder> {
  /// Optional attachment IDs to include with this message.
  @BuiltValueField(wireName: r'attachment_ids')
  BuiltList<String>? get attachmentIds;

  /// The message text to send to the assistant. Empty when `surface_action` carries the turn.
  @BuiltValueField(wireName: r'message')
  String get message;

  /// Answers an active surface after its turn ended. When present, `message` must be empty; the server builds the visible text.
  @BuiltValueField(wireName: r'surface_action')
  SurfaceActionRequest? get surfaceAction;

  ApiSendMessageRequest._();

  factory ApiSendMessageRequest(
      [void updates(ApiSendMessageRequestBuilder b)]) = _$ApiSendMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApiSendMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApiSendMessageRequest> get serializer =>
      _$ApiSendMessageRequestSerializer();
}

class _$ApiSendMessageRequestSerializer
    implements PrimitiveSerializer<ApiSendMessageRequest> {
  @override
  final Iterable<Type> types = const [
    ApiSendMessageRequest,
    _$ApiSendMessageRequest
  ];

  @override
  final String wireName = r'ApiSendMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApiSendMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.attachmentIds != null) {
      yield r'attachment_ids';
      yield serializers.serialize(
        object.attachmentIds,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    if (object.surfaceAction != null) {
      yield r'surface_action';
      yield serializers.serialize(
        object.surfaceAction,
        specifiedType: const FullType.nullable(SurfaceActionRequest),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ApiSendMessageRequest object, {
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
    required ApiSendMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'attachment_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.attachmentIds.replace(valueDes);
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'surface_action':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(SurfaceActionRequest),
          ) as SurfaceActionRequest?;
          if (valueDes == null) continue;
          result.surfaceAction.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApiSendMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApiSendMessageRequestBuilder();
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
