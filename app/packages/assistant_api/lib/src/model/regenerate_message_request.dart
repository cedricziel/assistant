//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'regenerate_message_request.g.dart';

/// Body for `POST /api/conversations/{id}/messages/{message_id}/regenerate`. The body may be omitted, which regenerates on the default model.
///
/// Properties:
/// * [model] - The model to answer with: the org's default model or an entry of `[llm] available_models`. It applies to this reply only.
@BuiltValue()
abstract class RegenerateMessageRequest
    implements
        Built<RegenerateMessageRequest, RegenerateMessageRequestBuilder> {
  /// The model to answer with: the org's default model or an entry of `[llm] available_models`. It applies to this reply only.
  @BuiltValueField(wireName: r'model')
  String? get model;

  RegenerateMessageRequest._();

  factory RegenerateMessageRequest(
          [void updates(RegenerateMessageRequestBuilder b)]) =
      _$RegenerateMessageRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RegenerateMessageRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RegenerateMessageRequest> get serializer =>
      _$RegenerateMessageRequestSerializer();
}

class _$RegenerateMessageRequestSerializer
    implements PrimitiveSerializer<RegenerateMessageRequest> {
  @override
  final Iterable<Type> types = const [
    RegenerateMessageRequest,
    _$RegenerateMessageRequest
  ];

  @override
  final String wireName = r'RegenerateMessageRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RegenerateMessageRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.model != null) {
      yield r'model';
      yield serializers.serialize(
        object.model,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    RegenerateMessageRequest object, {
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
    required RegenerateMessageRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'model':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.model = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RegenerateMessageRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RegenerateMessageRequestBuilder();
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
