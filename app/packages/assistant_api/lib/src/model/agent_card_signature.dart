//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'agent_card_signature.g.dart';

/// Represents a JWS signature of an AgentCard (RFC 7515).
///
/// Properties:
/// * [header] - The unprotected JWS header values.
/// * [protected] - The protected JWS header, base64url-encoded JSON object.
/// * [signature] - The computed signature, base64url-encoded.
@BuiltValue()
abstract class AgentCardSignature implements Built<AgentCardSignature, AgentCardSignatureBuilder> {
  /// The unprotected JWS header values.
  @BuiltValueField(wireName: r'header')
  BuiltMap<String, JsonObject?>? get header;

  /// The protected JWS header, base64url-encoded JSON object.
  @BuiltValueField(wireName: r'protected')
  String get protected;

  /// The computed signature, base64url-encoded.
  @BuiltValueField(wireName: r'signature')
  String get signature;

  AgentCardSignature._();

  factory AgentCardSignature([void updates(AgentCardSignatureBuilder b)]) = _$AgentCardSignature;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AgentCardSignatureBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AgentCardSignature> get serializer => _$AgentCardSignatureSerializer();
}

class _$AgentCardSignatureSerializer implements PrimitiveSerializer<AgentCardSignature> {
  @override
  final Iterable<Type> types = const [AgentCardSignature, _$AgentCardSignature];

  @override
  final String wireName = r'AgentCardSignature';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AgentCardSignature object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.header != null) {
      yield r'header';
      yield serializers.serialize(
        object.header,
        specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
      );
    }
    yield r'protected';
    yield serializers.serialize(
      object.protected,
      specifiedType: const FullType(String),
    );
    yield r'signature';
    yield serializers.serialize(
      object.signature,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    AgentCardSignature object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AgentCardSignatureBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'header':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(BuiltMap, [FullType(String), FullType.nullable(JsonObject)]),
          ) as BuiltMap<String, JsonObject?>?;
          if (valueDes == null) continue;
          result.header.replace(valueDes);
          break;
        case r'protected':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.protected = valueDes;
          break;
        case r'signature':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.signature = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AgentCardSignature deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AgentCardSignatureBuilder();
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

