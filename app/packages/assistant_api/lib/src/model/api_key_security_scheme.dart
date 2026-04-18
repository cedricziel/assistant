//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'api_key_security_scheme.g.dart';

/// Defines a security scheme using an API key.
///
/// Properties:
/// * [description] - An optional description for the security scheme.
/// * [location] - The location of the API key: \"query\", \"header\", or \"cookie\".
/// * [name] - The name of the header, query, or cookie parameter to be used.
@BuiltValue()
abstract class ApiKeySecurityScheme
    implements Built<ApiKeySecurityScheme, ApiKeySecuritySchemeBuilder> {
  /// An optional description for the security scheme.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// The location of the API key: \"query\", \"header\", or \"cookie\".
  @BuiltValueField(wireName: r'location')
  String get location;

  /// The name of the header, query, or cookie parameter to be used.
  @BuiltValueField(wireName: r'name')
  String get name;

  ApiKeySecurityScheme._();

  factory ApiKeySecurityScheme([void updates(ApiKeySecuritySchemeBuilder b)]) =
      _$ApiKeySecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApiKeySecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApiKeySecurityScheme> get serializer =>
      _$ApiKeySecuritySchemeSerializer();
}

class _$ApiKeySecuritySchemeSerializer
    implements PrimitiveSerializer<ApiKeySecurityScheme> {
  @override
  final Iterable<Type> types = const [
    ApiKeySecurityScheme,
    _$ApiKeySecurityScheme
  ];

  @override
  final String wireName = r'ApiKeySecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApiKeySecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'location';
    yield serializers.serialize(
      object.location,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApiKeySecurityScheme object, {
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
    required ApiKeySecuritySchemeBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.description = valueDes;
          break;
        case r'location':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.location = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApiKeySecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApiKeySecuritySchemeBuilder();
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
