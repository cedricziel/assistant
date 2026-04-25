//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'execute_command_request.g.dart';

/// Request body for `POST /api/conversations/{id}/command`.
///
/// Properties:
/// * [args] - Optional command arguments.
/// * [command] - Command name without the leading `/` (e.g. `\"model\"`).
@BuiltValue()
abstract class ExecuteCommandRequest
    implements Built<ExecuteCommandRequest, ExecuteCommandRequestBuilder> {
  /// Optional command arguments.
  @BuiltValueField(wireName: r'args')
  BuiltList<String>? get args;

  /// Command name without the leading `/` (e.g. `\"model\"`).
  @BuiltValueField(wireName: r'command')
  String get command;

  ExecuteCommandRequest._();

  factory ExecuteCommandRequest(
      [void updates(ExecuteCommandRequestBuilder b)]) = _$ExecuteCommandRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExecuteCommandRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExecuteCommandRequest> get serializer =>
      _$ExecuteCommandRequestSerializer();
}

class _$ExecuteCommandRequestSerializer
    implements PrimitiveSerializer<ExecuteCommandRequest> {
  @override
  final Iterable<Type> types = const [
    ExecuteCommandRequest,
    _$ExecuteCommandRequest
  ];

  @override
  final String wireName = r'ExecuteCommandRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExecuteCommandRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.args != null) {
      yield r'args';
      yield serializers.serialize(
        object.args,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'command';
    yield serializers.serialize(
      object.command,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExecuteCommandRequest object, {
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
    required ExecuteCommandRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'args':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.args.replace(valueDes);
          break;
        case r'command':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.command = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ExecuteCommandRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExecuteCommandRequestBuilder();
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
