//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'attachment_meta_response.g.dart';

/// Metadata returned after a successful upload.
///
/// Properties:
/// * [createdAt] 
/// * [filename] 
/// * [id] 
/// * [mimeType] 
/// * [sizeBytes] 
/// * [url] - URL to fetch the attachment content.
@BuiltValue()
abstract class AttachmentMetaResponse implements Built<AttachmentMetaResponse, AttachmentMetaResponseBuilder> {
  @BuiltValueField(wireName: r'created_at')
  DateTime get createdAt;

  @BuiltValueField(wireName: r'filename')
  String get filename;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'mime_type')
  String get mimeType;

  @BuiltValueField(wireName: r'size_bytes')
  int get sizeBytes;

  /// URL to fetch the attachment content.
  @BuiltValueField(wireName: r'url')
  String get url;

  AttachmentMetaResponse._();

  factory AttachmentMetaResponse([void updates(AttachmentMetaResponseBuilder b)]) = _$AttachmentMetaResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AttachmentMetaResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AttachmentMetaResponse> get serializer => _$AttachmentMetaResponseSerializer();
}

class _$AttachmentMetaResponseSerializer implements PrimitiveSerializer<AttachmentMetaResponse> {
  @override
  final Iterable<Type> types = const [AttachmentMetaResponse, _$AttachmentMetaResponse];

  @override
  final String wireName = r'AttachmentMetaResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AttachmentMetaResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(DateTime),
    );
    yield r'filename';
    yield serializers.serialize(
      object.filename,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'mime_type';
    yield serializers.serialize(
      object.mimeType,
      specifiedType: const FullType(String),
    );
    yield r'size_bytes';
    yield serializers.serialize(
      object.sizeBytes,
      specifiedType: const FullType(int),
    );
    yield r'url';
    yield serializers.serialize(
      object.url,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AttachmentMetaResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AttachmentMetaResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(DateTime),
          ) as DateTime;
          result.createdAt = valueDes;
          break;
        case r'filename':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.filename = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'mime_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.mimeType = valueDes;
          break;
        case r'size_bytes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.sizeBytes = valueDes;
          break;
        case r'url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.url = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AttachmentMetaResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AttachmentMetaResponseBuilder();
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

