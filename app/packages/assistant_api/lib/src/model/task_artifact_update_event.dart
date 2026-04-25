//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/artifact.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'task_artifact_update_event.g.dart';

/// A task delta where an artifact has been generated.
///
/// Properties:
/// * [append] - If true, append content to a previously sent artifact with the same ID.
/// * [artifact] - The artifact that was generated or updated.
/// * [contextId] - The ID of the context that this task belongs to.
/// * [lastChunk] - If true, this is the final chunk of the artifact.
/// * [metadata] - Metadata associated with the artifact update.
/// * [taskId] - The ID of the task for this artifact.
@BuiltValue()
abstract class TaskArtifactUpdateEvent
    implements Built<TaskArtifactUpdateEvent, TaskArtifactUpdateEventBuilder> {
  /// If true, append content to a previously sent artifact with the same ID.
  @BuiltValueField(wireName: r'append')
  bool? get append;

  /// The artifact that was generated or updated.
  @BuiltValueField(wireName: r'artifact')
  Artifact get artifact;

  /// The ID of the context that this task belongs to.
  @BuiltValueField(wireName: r'contextId')
  String get contextId;

  /// If true, this is the final chunk of the artifact.
  @BuiltValueField(wireName: r'lastChunk')
  bool? get lastChunk;

  /// Metadata associated with the artifact update.
  @BuiltValueField(wireName: r'metadata')
  BuiltMap<String, JsonObject?>? get metadata;

  /// The ID of the task for this artifact.
  @BuiltValueField(wireName: r'taskId')
  String get taskId;

  TaskArtifactUpdateEvent._();

  factory TaskArtifactUpdateEvent(
          [void updates(TaskArtifactUpdateEventBuilder b)]) =
      _$TaskArtifactUpdateEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TaskArtifactUpdateEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TaskArtifactUpdateEvent> get serializer =>
      _$TaskArtifactUpdateEventSerializer();
}

class _$TaskArtifactUpdateEventSerializer
    implements PrimitiveSerializer<TaskArtifactUpdateEvent> {
  @override
  final Iterable<Type> types = const [
    TaskArtifactUpdateEvent,
    _$TaskArtifactUpdateEvent
  ];

  @override
  final String wireName = r'TaskArtifactUpdateEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TaskArtifactUpdateEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.append != null) {
      yield r'append';
      yield serializers.serialize(
        object.append,
        specifiedType: const FullType(bool),
      );
    }
    yield r'artifact';
    yield serializers.serialize(
      object.artifact,
      specifiedType: const FullType(Artifact),
    );
    yield r'contextId';
    yield serializers.serialize(
      object.contextId,
      specifiedType: const FullType(String),
    );
    if (object.lastChunk != null) {
      yield r'lastChunk';
      yield serializers.serialize(
        object.lastChunk,
        specifiedType: const FullType(bool),
      );
    }
    if (object.metadata != null) {
      yield r'metadata';
      yield serializers.serialize(
        object.metadata,
        specifiedType: const FullType.nullable(
            BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    yield r'taskId';
    yield serializers.serialize(
      object.taskId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TaskArtifactUpdateEvent object, {
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
    required TaskArtifactUpdateEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'append':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.append = valueDes;
          break;
        case r'artifact':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(Artifact),
          ) as Artifact;
          result.artifact.replace(valueDes);
          break;
        case r'contextId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.contextId = valueDes;
          break;
        case r'lastChunk':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.lastChunk = valueDes;
          break;
        case r'metadata':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(
                BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.metadata.replace(valueDes);
          break;
        case r'taskId':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.taskId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TaskArtifactUpdateEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TaskArtifactUpdateEventBuilder();
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
