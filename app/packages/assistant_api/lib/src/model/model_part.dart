//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'model_part.g.dart';

/// `Part` represents a container for a section of communication content.  Parts can be purely textual, a file (image, video, etc.), or a structured data blob (JSON).
///
/// Properties:
/// * [data] - Arbitrary structured data as a JSON value.
/// * [filename] - An optional filename for the file.
/// * [mediaType] - The MIME type of the part content.
/// * [metadata] - Metadata associated with this part.
/// * [raw] - Raw byte content of a file, base64-encoded in JSON.
/// * [text] - The string content of a text part.
/// * [url] - A URL pointing to the file's content.
@BuiltValue()
abstract class ModelPart implements Built<ModelPart, ModelPartBuilder> {
  /// Arbitrary structured data as a JSON value.
  @BuiltValueField(wireName: r'data')
  JsonObject? get data;

  /// An optional filename for the file.
  @BuiltValueField(wireName: r'filename')
  String? get filename;

  /// The MIME type of the part content.
  @BuiltValueField(wireName: r'mediaType')
  String? get mediaType;

  /// Metadata associated with this part.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// Raw byte content of a file, base64-encoded in JSON.
  @BuiltValueField(wireName: r'raw')
  String? get raw;

  /// The string content of a text part.
  @BuiltValueField(wireName: r'text')
  String? get text;

  /// A URL pointing to the file's content.
  @BuiltValueField(wireName: r'url')
  String? get url;

  ModelPart._();

  factory ModelPart([void updates(ModelPartBuilder b)]) = _$ModelPart;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ModelPartBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ModelPart> get serializer => _$ModelPartSerializer();
}

class _$ModelPartSerializer implements PrimitiveSerializer<ModelPart> {
  @override
  final Iterable<Type> types = const [ModelPart, _$ModelPart];

  @override
  final String wireName = r'ModelPart';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ModelPart object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.data != null) {
      yield r'data';
      yield serializers.serialize(
        object.data,
        specifiedType: const FullType(JsonObject),
      );
    }
    if (object.filename != null) {
      yield r'filename';
      yield serializers.serialize(
        object.filename,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.mediaType != null) {
      yield r'mediaType';
      yield serializers.serialize(
        object.mediaType,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    if (object.raw != null) {
      yield r'raw';
      yield serializers.serialize(
        object.raw,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.text != null) {
      yield r'text';
      yield serializers.serialize(
        object.text,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.url != null) {
      yield r'url';
      yield serializers.serialize(
        object.url,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ModelPart object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ModelPartBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'data':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(JsonObject),
          ) as JsonObject;
          result.data = valueDes;
          break;
        case r'filename':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.filename = valueDes;
          break;
        case r'mediaType':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.mediaType = valueDes;
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'raw':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.raw = valueDes;
          break;
        case r'text':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.text = valueDes;
          break;
        case r'url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  ModelPart deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ModelPartBuilder();
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

