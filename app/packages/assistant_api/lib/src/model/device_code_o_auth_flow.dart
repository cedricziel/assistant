//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'device_code_o_auth_flow.g.dart';

/// OAuth 2.0 Device Code flow configuration (RFC 8628).
///
/// Properties:
/// * [deviceAuthorizationUrl] - The device authorization endpoint URL.
/// * [refreshUrl] - The URL for obtaining refresh tokens.
/// * [scopes] - Available scopes for the OAuth2 security scheme.
/// * [tokenUrl] - The token URL.
@BuiltValue()
abstract class DeviceCodeOAuthFlow
    implements Built<DeviceCodeOAuthFlow, DeviceCodeOAuthFlowBuilder> {
  /// The device authorization endpoint URL.
  @BuiltValueField(wireName: r'deviceAuthorizationUrl')
  String get deviceAuthorizationUrl;

  /// The URL for obtaining refresh tokens.
  @BuiltValueField(wireName: r'refreshUrl')
  String? get refreshUrl;

  /// Available scopes for the OAuth2 security scheme.
  @BuiltValueField(wireName: r'scopes')
  BuiltMap<String, String> get scopes;

  /// The token URL.
  @BuiltValueField(wireName: r'tokenUrl')
  String get tokenUrl;

  DeviceCodeOAuthFlow._();

  factory DeviceCodeOAuthFlow([void updates(DeviceCodeOAuthFlowBuilder b)]) =
      _$DeviceCodeOAuthFlow;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(DeviceCodeOAuthFlowBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<DeviceCodeOAuthFlow> get serializer =>
      _$DeviceCodeOAuthFlowSerializer();
}

class _$DeviceCodeOAuthFlowSerializer
    implements PrimitiveSerializer<DeviceCodeOAuthFlow> {
  @override
  final Iterable<Type> types = const [
    DeviceCodeOAuthFlow,
    _$DeviceCodeOAuthFlow
  ];

  @override
  final String wireName = r'DeviceCodeOAuthFlow';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    DeviceCodeOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'deviceAuthorizationUrl';
    yield serializers.serialize(
      object.deviceAuthorizationUrl,
      specifiedType: const FullType(String),
    );
    if (object.refreshUrl != null) {
      yield r'refreshUrl';
      yield serializers.serialize(
        object.refreshUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'scopes';
    yield serializers.serialize(
      object.scopes,
      specifiedType:
          const FullType(BuiltMap, [FullType(String), FullType(String)]),
    );
    yield r'tokenUrl';
    yield serializers.serialize(
      object.tokenUrl,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    DeviceCodeOAuthFlow object, {
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
    required DeviceCodeOAuthFlowBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'deviceAuthorizationUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.deviceAuthorizationUrl = valueDes;
          break;
        case r'refreshUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.refreshUrl = valueDes;
          break;
        case r'scopes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltMap, [FullType(String), FullType(String)]),
          ) as BuiltMap<String, String>;
          result.scopes.replace(valueDes);
          break;
        case r'tokenUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.tokenUrl = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  DeviceCodeOAuthFlow deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = DeviceCodeOAuthFlowBuilder();
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
