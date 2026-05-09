//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'error_body.g.dart';

/// Standard error envelope returned for every 4xx/5xx response from `/api/...` endpoints. Matches the project convention documented in `AGENTS.md`: `{\"error\": \"...message...\"}`.  OAuth2 endpoints under `/oauth/...` use [`crate::oauth::OAuthErrorResponse`] (RFC 6749 §5.2) and are intentionally exempt.
///
/// Properties:
/// * [error] - Human-readable error message.
@BuiltValue()
abstract class ErrorBody implements Built<ErrorBody, ErrorBodyBuilder> {
  /// Human-readable error message.
  @BuiltValueField(wireName: r'error')
  String get error;

  ErrorBody._();

  factory ErrorBody([void updates(ErrorBodyBuilder b)]) = _$ErrorBody;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ErrorBodyBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ErrorBody> get serializer => _$ErrorBodySerializer();
}

class _$ErrorBodySerializer implements PrimitiveSerializer<ErrorBody> {
  @override
  final Iterable<Type> types = const [ErrorBody, _$ErrorBody];

  @override
  final String wireName = r'ErrorBody';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ErrorBody object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'error';
    yield serializers.serialize(
      object.error,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ErrorBody object, {
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
    required ErrorBodyBuilder result,
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
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ErrorBody deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ErrorBodyBuilder();
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
