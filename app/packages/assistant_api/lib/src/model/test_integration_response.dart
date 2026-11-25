//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/integration_status.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'test_integration_response.g.dart';

/// The outcome of `POST /api/integrations/{id}/test`.
///
/// Properties:
/// * [error] - Why the connection failed.
/// * [lastConnectedAt]
/// * [ok] - Whether a connection was made and its tools listed.
/// * [status] - The caller's status after this test.
/// * [toolCount]
/// * [toolNames] - The tools the server offers, sorted. Empty when the test failed.
@BuiltValue()
abstract class TestIntegrationResponse
    implements Built<TestIntegrationResponse, TestIntegrationResponseBuilder> {
  /// Why the connection failed.
  @BuiltValueField(wireName: r'error')
  String? get error;

  @BuiltValueField(wireName: r'last_connected_at')
  DateTime? get lastConnectedAt;

  /// Whether a connection was made and its tools listed.
  @BuiltValueField(wireName: r'ok')
  bool get ok;

  /// The caller's status after this test.
  @BuiltValueField(wireName: r'status')
  IntegrationStatus get status;
  // enum statusEnum {  ready,  needs_connection,  auth_expired,  unreachable,  blocked_by_policy,  disabled,  };

  @BuiltValueField(wireName: r'tool_count')
  int? get toolCount;

  /// The tools the server offers, sorted. Empty when the test failed.
  @BuiltValueField(wireName: r'tool_names')
  BuiltList<String> get toolNames;

  TestIntegrationResponse._();

  factory TestIntegrationResponse(
          [void updates(TestIntegrationResponseBuilder b)]) =
      _$TestIntegrationResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TestIntegrationResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TestIntegrationResponse> get serializer =>
      _$TestIntegrationResponseSerializer();
}

class _$TestIntegrationResponseSerializer
    implements PrimitiveSerializer<TestIntegrationResponse> {
  @override
  final Iterable<Type> types = const [
    TestIntegrationResponse,
    _$TestIntegrationResponse
  ];

  @override
  final String wireName = r'TestIntegrationResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TestIntegrationResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.error != null) {
      yield r'error';
      yield serializers.serialize(
        object.error,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.lastConnectedAt != null) {
      yield r'last_connected_at';
      yield serializers.serialize(
        object.lastConnectedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
    yield r'ok';
    yield serializers.serialize(
      object.ok,
      specifiedType: const FullType(bool),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(IntegrationStatus),
    );
    if (object.toolCount != null) {
      yield r'tool_count';
      yield serializers.serialize(
        object.toolCount,
        specifiedType: const FullType.nullable(int),
      );
    }
    yield r'tool_names';
    yield serializers.serialize(
      object.toolNames,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TestIntegrationResponse object, {
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
    required TestIntegrationResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'error':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.error = valueDes;
          break;
        case r'last_connected_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.lastConnectedAt = valueDes;
          break;
        case r'ok':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.ok = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(IntegrationStatus),
          ) as IntegrationStatus;
          result.status = valueDes;
          break;
        case r'tool_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.toolCount = valueDes;
          break;
        case r'tool_names':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.toolNames.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TestIntegrationResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TestIntegrationResponseBuilder();
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
