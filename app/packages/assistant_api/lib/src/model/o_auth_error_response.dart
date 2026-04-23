//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'o_auth_error_response.g.dart';

/// OAuth2 error response — OpenAPI schema type.
///
/// Properties:
/// * [error] - Error code (e.g. `invalid_grant`, `invalid_request`).
/// * [errorDescription] - Human-readable description of the error.
@BuiltValue()
abstract class OAuthErrorResponse
    implements Built<OAuthErrorResponse, OAuthErrorResponseBuilder> {
  /// Error code (e.g. `invalid_grant`, `invalid_request`).
  @BuiltValueField(wireName: r'error')
  String get error;

  /// Human-readable description of the error.
  @BuiltValueField(wireName: r'error_description')
  String get errorDescription;

  OAuthErrorResponse._();

  factory OAuthErrorResponse([void updates(OAuthErrorResponseBuilder b)]) =
      _$OAuthErrorResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OAuthErrorResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OAuthErrorResponse> get serializer =>
      _$OAuthErrorResponseSerializer();
}

class _$OAuthErrorResponseSerializer
    implements PrimitiveSerializer<OAuthErrorResponse> {
  @override
  final Iterable<Type> types = const [OAuthErrorResponse, _$OAuthErrorResponse];

  @override
  final String wireName = r'OAuthErrorResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OAuthErrorResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'error';
    yield serializers.serialize(
      object.error,
      specifiedType: const FullType(String),
    );
    yield r'error_description';
    yield serializers.serialize(
      object.errorDescription,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    OAuthErrorResponse object, {
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
    required OAuthErrorResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'error':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.error = valueDes;
          break;
        case r'error_description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.errorDescription = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OAuthErrorResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OAuthErrorResponseBuilder();
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
