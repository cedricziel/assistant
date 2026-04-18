//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/message.dart';
import 'package:assistant_api/src/model/task_artifact_update_event.dart';
import 'package:assistant_api/src/model/task_status_update_event.dart';
import 'package:assistant_api/src/model/task.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'stream_response.g.dart';

/// A wrapper for streaming operations encapsulating different response types.  In SSE, each chunk is one `StreamResponse` serialized as JSON.
///
/// Properties:
/// * [artifactUpdate] - An event indicating a task artifact update.
/// * [message] - A Message object containing a message from the agent.
/// * [statusUpdate] - An event indicating a task status update.
/// * [task] - A Task object containing the current state of the task.
@BuiltValue()
abstract class StreamResponse
    implements Built<StreamResponse, StreamResponseBuilder> {
  /// An event indicating a task artifact update.
  @BuiltValueField(wireName: r'artifactUpdate')
  TaskArtifactUpdateEvent? get artifactUpdate;

  /// A Message object containing a message from the agent.
  @BuiltValueField(wireName: r'message')
  Message? get message;

  /// An event indicating a task status update.
  @BuiltValueField(wireName: r'statusUpdate')
  TaskStatusUpdateEvent? get statusUpdate;

  /// A Task object containing the current state of the task.
  @BuiltValueField(wireName: r'task')
  Task? get task;

  StreamResponse._();

  factory StreamResponse([void updates(StreamResponseBuilder b)]) =
      _$StreamResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(StreamResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<StreamResponse> get serializer =>
      _$StreamResponseSerializer();
}

class _$StreamResponseSerializer
    implements PrimitiveSerializer<StreamResponse> {
  @override
  final Iterable<Type> types = const [StreamResponse, _$StreamResponse];

  @override
  final String wireName = r'StreamResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    StreamResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.artifactUpdate != null) {
      yield r'artifactUpdate';
      yield serializers.serialize(
        object.artifactUpdate,
        specifiedType: const FullType.nullable(TaskArtifactUpdateEvent),
      );
    }
    if (object.message != null) {
      yield r'message';
      yield serializers.serialize(
        object.message,
        specifiedType: const FullType.nullable(Message),
      );
    }
    if (object.statusUpdate != null) {
      yield r'statusUpdate';
      yield serializers.serialize(
        object.statusUpdate,
        specifiedType: const FullType.nullable(TaskStatusUpdateEvent),
      );
    }
    if (object.task != null) {
      yield r'task';
      yield serializers.serialize(
        object.task,
        specifiedType: const FullType.nullable(Task),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    StreamResponse object, {
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
    required StreamResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'artifactUpdate':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(TaskArtifactUpdateEvent),
          ) as TaskArtifactUpdateEvent?;
          if (valueDes == null) continue;
          result.artifactUpdate.replace(valueDes);
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(Message),
          ) as Message?;
          if (valueDes == null) continue;
          result.message.replace(valueDes);
          break;
        case r'statusUpdate':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(TaskStatusUpdateEvent),
          ) as TaskStatusUpdateEvent?;
          if (valueDes == null) continue;
          result.statusUpdate.replace(valueDes);
          break;
        case r'task':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(Task),
          ) as Task?;
          if (valueDes == null) continue;
          result.task.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  StreamResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = StreamResponseBuilder();
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
