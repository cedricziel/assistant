//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/model_part.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'artifact.g.dart';

/// Artifacts represent task outputs.
///
/// Properties:
/// * [artifactId] - Unique identifier (UUID) for the artifact, unique within a task.
/// * [description] - A human-readable description of the artifact.
/// * [extensions] - The URIs of extensions present or contributing to this artifact.
/// * [metadata] - Metadata included with the artifact.
/// * [name] - A human-readable name for the artifact.
/// * [parts] - The content of the artifact. Must contain at least one part.
@BuiltValue()
abstract class Artifact implements Built<Artifact, ArtifactBuilder> {
  /// Unique identifier (UUID) for the artifact, unique within a task.
  @BuiltValueField(wireName: r'artifactId')
  String get artifactId;

  /// A human-readable description of the artifact.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// The URIs of extensions present or contributing to this artifact.
  @BuiltValueField(wireName: r'extensions')
  BuiltList<String>? get extensions;

  /// Metadata included with the artifact.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// A human-readable name for the artifact.
  @BuiltValueField(wireName: r'name')
  String? get name;

  /// The content of the artifact. Must contain at least one part.
  @BuiltValueField(wireName: r'parts')
  BuiltList<ModelPart> get parts;

  Artifact._();

  factory Artifact([void updates(ArtifactBuilder b)]) = _$Artifact;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ArtifactBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<Artifact> get serializer => _$ArtifactSerializer();
}

class _$ArtifactSerializer implements PrimitiveSerializer<Artifact> {
  @override
  final Iterable<Type> types = const [Artifact, _$Artifact];

  @override
  final String wireName = r'Artifact';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    Artifact object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'artifactId';
    yield serializers.serialize(
      object.artifactId,
      specifiedType: const FullType(String),
    );
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.extensions != null) {
      yield r'extensions';
      yield serializers.serialize(
        object.extensions,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    if (object.name != null) {
      yield r'name';
      yield serializers.serialize(
        object.name,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'parts';
    yield serializers.serialize(
      object.parts,
      specifiedType: const FullType(BuiltList, [FullType(ModelPart)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    Artifact object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ArtifactBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'artifactId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.artifactId = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.description = valueDes;
          break;
        case r'extensions':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.extensions.replace(valueDes);
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.name = valueDes;
          break;
        case r'parts':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ModelPart)]),
          ) as BuiltList<ModelPart>;
          result.parts.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  Artifact deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ArtifactBuilder();
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

