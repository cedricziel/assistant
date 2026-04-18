//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'password_o_auth_flow.g.dart';

/// Deprecated: Use Authorization Code + PKCE or Device Code.
///
/// Properties:
/// * [refreshUrl] - The URL for obtaining refresh tokens.
/// * [scopes] - Available scopes.
/// * [tokenUrl] - The token URL.
@BuiltValue()
abstract class PasswordOAuthFlow
    implements Built<PasswordOAuthFlow, PasswordOAuthFlowBuilder> {
  /// The URL for obtaining refresh tokens.
  @BuiltValueField(wireName: r'refreshUrl')
  String? get refreshUrl;

  /// Available scopes.
  @BuiltValueField(wireName: r'scopes')
  BuiltMap<String, String>? get scopes;

  /// The token URL.
  @BuiltValueField(wireName: r'tokenUrl')
  String? get tokenUrl;

  PasswordOAuthFlow._();

  factory PasswordOAuthFlow([void updates(PasswordOAuthFlowBuilder b)]) =
      _$PasswordOAuthFlow;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PasswordOAuthFlowBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PasswordOAuthFlow> get serializer =>
      _$PasswordOAuthFlowSerializer();
}

class _$PasswordOAuthFlowSerializer
    implements PrimitiveSerializer<PasswordOAuthFlow> {
  @override
  final Iterable<Type> types = const [PasswordOAuthFlow, _$PasswordOAuthFlow];

  @override
  final String wireName = r'PasswordOAuthFlow';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PasswordOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.refreshUrl != null) {
      yield r'refreshUrl';
      yield serializers.serialize(
        object.refreshUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.scopes != null) {
      yield r'scopes';
      yield serializers.serialize(
        object.scopes,
        specifiedType:
            const FullType(BuiltMap, [FullType(String), FullType(String)]),
      );
    }
    if (object.tokenUrl != null) {
      yield r'tokenUrl';
      yield serializers.serialize(
        object.tokenUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    PasswordOAuthFlow object, {
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
    required PasswordOAuthFlowBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
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
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
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
  PasswordOAuthFlow deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PasswordOAuthFlowBuilder();
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
