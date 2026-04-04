//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'http_auth_security_scheme.g.dart';

/// Defines a security scheme using HTTP authentication.
///
/// Properties:
/// * [bearerFormat] - A hint for how the bearer token is formatted (e.g., \"JWT\").
/// * [description] - An optional description for the security scheme.
/// * [scheme] - The HTTP Authentication scheme (e.g., \"Bearer\").
@BuiltValue()
abstract class HttpAuthSecurityScheme implements Built<HttpAuthSecurityScheme, HttpAuthSecuritySchemeBuilder> {
  /// A hint for how the bearer token is formatted (e.g., \"JWT\").
  @BuiltValueField(wireName: r'bearerFormat')
  String? get bearerFormat;

  /// An optional description for the security scheme.
  @BuiltValueField(wireName: r'description')
  String? get description;

  /// The HTTP Authentication scheme (e.g., \"Bearer\").
  @BuiltValueField(wireName: r'scheme')
  String get scheme;

  HttpAuthSecurityScheme._();

  factory HttpAuthSecurityScheme([void updates(HttpAuthSecuritySchemeBuilder b)]) = _$HttpAuthSecurityScheme;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(HttpAuthSecuritySchemeBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<HttpAuthSecurityScheme> get serializer => _$HttpAuthSecuritySchemeSerializer();
}

class _$HttpAuthSecuritySchemeSerializer implements PrimitiveSerializer<HttpAuthSecurityScheme> {
  @override
  final Iterable<Type> types = const [HttpAuthSecurityScheme, _$HttpAuthSecurityScheme];

  @override
  final String wireName = r'HttpAuthSecurityScheme';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    HttpAuthSecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.bearerFormat != null) {
      yield r'bearerFormat';
      yield serializers.serialize(
        object.bearerFormat,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'scheme';
    yield serializers.serialize(
      object.scheme,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    HttpAuthSecurityScheme object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required HttpAuthSecuritySchemeBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'bearerFormat':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.bearerFormat = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.description = valueDes;
          break;
        case r'scheme':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.scheme = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  HttpAuthSecurityScheme deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = HttpAuthSecuritySchemeBuilder();
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

