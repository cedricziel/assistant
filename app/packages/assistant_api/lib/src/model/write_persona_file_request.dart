//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'write_persona_file_request.g.dart';

/// Body for `PUT /api/personas/{id}/files/{filename}`.
///
/// Properties:
/// * [content]
@BuiltValue()
abstract class WritePersonaFileRequest
    implements Built<WritePersonaFileRequest, WritePersonaFileRequestBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  WritePersonaFileRequest._();

  factory WritePersonaFileRequest(
          [void updates(WritePersonaFileRequestBuilder b)]) =
      _$WritePersonaFileRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(WritePersonaFileRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<WritePersonaFileRequest> get serializer =>
      _$WritePersonaFileRequestSerializer();
}

class _$WritePersonaFileRequestSerializer
    implements PrimitiveSerializer<WritePersonaFileRequest> {
  @override
  final Iterable<Type> types = const [
    WritePersonaFileRequest,
    _$WritePersonaFileRequest
  ];

  @override
  final String wireName = r'WritePersonaFileRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    WritePersonaFileRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    WritePersonaFileRequest object, {
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
    required WritePersonaFileRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  WritePersonaFileRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = WritePersonaFileRequestBuilder();
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
