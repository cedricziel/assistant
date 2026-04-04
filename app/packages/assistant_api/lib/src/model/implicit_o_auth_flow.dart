//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'implicit_o_auth_flow.g.dart';

/// Deprecated: Use Authorization Code + PKCE instead.
///
/// Properties:
/// * [authorizationUrl] - The authorization URL.
/// * [refreshUrl] - The URL for obtaining refresh tokens.
/// * [scopes] - Available scopes.
@BuiltValue()
abstract class ImplicitOAuthFlow implements Built<ImplicitOAuthFlow, ImplicitOAuthFlowBuilder> {
  /// The authorization URL.
  @BuiltValueField(wireName: r'authorizationUrl')
  String? get authorizationUrl;

  /// The URL for obtaining refresh tokens.
  @BuiltValueField(wireName: r'refreshUrl')
  String? get refreshUrl;

  /// Available scopes.
  @BuiltValueField(wireName: r'scopes')
  BuiltMap<String, String>? get scopes;

  ImplicitOAuthFlow._();

  factory ImplicitOAuthFlow([void updates(ImplicitOAuthFlowBuilder b)]) = _$ImplicitOAuthFlow;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ImplicitOAuthFlowBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ImplicitOAuthFlow> get serializer => _$ImplicitOAuthFlowSerializer();
}

class _$ImplicitOAuthFlowSerializer implements PrimitiveSerializer<ImplicitOAuthFlow> {
  @override
  final Iterable<Type> types = const [ImplicitOAuthFlow, _$ImplicitOAuthFlow];

  @override
  final String wireName = r'ImplicitOAuthFlow';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ImplicitOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authorizationUrl != null) {
      yield r'authorizationUrl';
      yield serializers.serialize(
        object.authorizationUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
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
        specifiedType: const FullType(BuiltMap, [FullType(String), FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ImplicitOAuthFlow object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ImplicitOAuthFlowBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authorizationUrl':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.authorizationUrl = valueDes;
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
            specifiedType: const FullType(BuiltMap, [FullType(String), FullType(String)]),
          ) as BuiltMap<String, String>;
          result.scopes.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ImplicitOAuthFlow deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ImplicitOAuthFlowBuilder();
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

