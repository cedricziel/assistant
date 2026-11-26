//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'protected_resource_metadata.g.dart';

/// OAuth 2.0 protected resource metadata for `/mcp` (RFC 9728).
///
/// Properties:
/// * [authorizationServers] - Authorization servers that issue tokens for this resource.
/// * [bearerMethodsSupported]
/// * [resource] - Canonical URL of the MCP endpoint; tokens must be issued for it.
/// * [scopesSupported]
@BuiltValue()
abstract class ProtectedResourceMetadata
    implements
        Built<ProtectedResourceMetadata, ProtectedResourceMetadataBuilder> {
  /// Authorization servers that issue tokens for this resource.
  @BuiltValueField(wireName: r'authorization_servers')
  BuiltList<String> get authorizationServers;

  @BuiltValueField(wireName: r'bearer_methods_supported')
  BuiltList<String> get bearerMethodsSupported;

  /// Canonical URL of the MCP endpoint; tokens must be issued for it.
  @BuiltValueField(wireName: r'resource')
  String get resource;

  @BuiltValueField(wireName: r'scopes_supported')
  BuiltList<String> get scopesSupported;

  ProtectedResourceMetadata._();

  factory ProtectedResourceMetadata(
          [void updates(ProtectedResourceMetadataBuilder b)]) =
      _$ProtectedResourceMetadata;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ProtectedResourceMetadataBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ProtectedResourceMetadata> get serializer =>
      _$ProtectedResourceMetadataSerializer();
}

class _$ProtectedResourceMetadataSerializer
    implements PrimitiveSerializer<ProtectedResourceMetadata> {
  @override
  final Iterable<Type> types = const [
    ProtectedResourceMetadata,
    _$ProtectedResourceMetadata
  ];

  @override
  final String wireName = r'ProtectedResourceMetadata';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ProtectedResourceMetadata object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'authorization_servers';
    yield serializers.serialize(
      object.authorizationServers,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'bearer_methods_supported';
    yield serializers.serialize(
      object.bearerMethodsSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'resource';
    yield serializers.serialize(
      object.resource,
      specifiedType: const FullType(String),
    );
    yield r'scopes_supported';
    yield serializers.serialize(
      object.scopesSupported,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ProtectedResourceMetadata object, {
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
    required ProtectedResourceMetadataBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authorization_servers':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.authorizationServers.replace(valueDes);
          break;
        case r'bearer_methods_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.bearerMethodsSupported.replace(valueDes);
          break;
        case r'resource':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.resource = valueDes;
          break;
        case r'scopes_supported':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.scopesSupported.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ProtectedResourceMetadata deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ProtectedResourceMetadataBuilder();
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
