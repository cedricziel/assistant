//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'edit_message_request.g.dart';

/// Body for `POST /api/conversations/{id}/messages/{message_id}/edit`.
///
/// Properties:
/// * [message] - The new text. The edited message's attachments carry forward.
@BuiltValue()
abstract class EditMessageRequest
    implements Built<EditMessageRequest, EditMessageRequestBuilder> {
  /// The new text. The edited message's attachments carry forward.
  @BuiltValueField(wireName: r'message')
  String get message;

  EditMessageRequest._();

  factory EditMessageRequest([void updates(EditMessageRequestBuilder b)]) =
      _$EditMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(EditMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<EditMessageRequest> get serializer =>
      _$EditMessageRequestSerializer();
}

class _$EditMessageRequestSerializer
    implements PrimitiveSerializer<EditMessageRequest> {
  @override
  final Iterable<Type> types = const [EditMessageRequest, _$EditMessageRequest];

  @override
  final String wireName = r'EditMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    EditMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    EditMessageRequest object, {
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
    required EditMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  EditMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = EditMessageRequestBuilder();
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
