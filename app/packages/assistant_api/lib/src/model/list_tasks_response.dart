//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/task.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'list_tasks_response.g.dart';

/// Response for the `ListTasks` method.
///
/// Properties:
/// * [nextPageToken] - A token to retrieve the next page of results.
/// * [pageSize] - The page size used for this response.
/// * [tasks] - Tasks matching the specified criteria.
/// * [totalSize] - Total number of tasks available (before pagination).
@BuiltValue()
abstract class ListTasksResponse implements Built<ListTasksResponse, ListTasksResponseBuilder> {
  /// A token to retrieve the next page of results.
  @BuiltValueField(wireName: r'nextPageToken')
  String get nextPageToken;

  /// The page size used for this response.
  @BuiltValueField(wireName: r'pageSize')
  int get pageSize;

  /// Tasks matching the specified criteria.
  @BuiltValueField(wireName: r'tasks')
  BuiltList<Task> get tasks;

  /// Total number of tasks available (before pagination).
  @BuiltValueField(wireName: r'totalSize')
  int get totalSize;

  ListTasksResponse._();

  factory ListTasksResponse([void updates(ListTasksResponseBuilder b)]) = _$ListTasksResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ListTasksResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ListTasksResponse> get serializer => _$ListTasksResponseSerializer();
}

class _$ListTasksResponseSerializer implements PrimitiveSerializer<ListTasksResponse> {
  @override
  final Iterable<Type> types = const [ListTasksResponse, _$ListTasksResponse];

  @override
  final String wireName = r'ListTasksResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ListTasksResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'nextPageToken';
    yield serializers.serialize(
      object.nextPageToken,
      specifiedType: const FullType(String),
    );
    yield r'pageSize';
    yield serializers.serialize(
      object.pageSize,
      specifiedType: const FullType(int),
    );
    yield r'tasks';
    yield serializers.serialize(
      object.tasks,
      specifiedType: const FullType(BuiltList, [FullType(Task)]),
    );
    yield r'totalSize';
    yield serializers.serialize(
      object.totalSize,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ListTasksResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ListTasksResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'nextPageToken':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.nextPageToken = valueDes;
          break;
        case r'pageSize':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.pageSize = valueDes;
          break;
        case r'tasks':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(Task)]),
          ) as BuiltList<Task>;
          result.tasks.replace(valueDes);
          break;
        case r'totalSize':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.totalSize = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ListTasksResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ListTasksResponseBuilder();
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

