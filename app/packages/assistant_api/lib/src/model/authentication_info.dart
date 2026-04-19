//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'authentication_info.g.dart';

/// Defines authentication details, used for push notifications.
///
/// Properties:
/// * [credentials] - Credentials. Format depends on the scheme.
/// * [scheme] - HTTP Authentication Scheme (e.g., \"Bearer\", \"Basic\", \"Digest\").
@BuiltValue()
abstract class AuthenticationInfo implements Built<AuthenticationInfo, AuthenticationInfoBuilder> {
  /// Credentials. Format depends on the scheme.
  @BuiltValueField(wireName: r'credentials')
  String? get credentials;

  /// HTTP Authentication Scheme (e.g., \"Bearer\", \"Basic\", \"Digest\").
  @BuiltValueField(wireName: r'scheme')
  String get scheme;

  AuthenticationInfo._();

  factory AuthenticationInfo([void updates(AuthenticationInfoBuilder b)]) = _$AuthenticationInfo;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AuthenticationInfoBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AuthenticationInfo> get serializer => _$AuthenticationInfoSerializer();
}

class _$AuthenticationInfoSerializer implements PrimitiveSerializer<AuthenticationInfo> {
  @override
  final Iterable<Type> types = const [AuthenticationInfo, _$AuthenticationInfo];

  @override
  final String wireName = r'AuthenticationInfo';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AuthenticationInfo object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.credentials != null) {
      yield r'credentials';
      yield serializers.serialize(
        object.credentials,
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
    AuthenticationInfo object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AuthenticationInfoBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'credentials':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.credentials = valueDes;
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
  AuthenticationInfo deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AuthenticationInfoBuilder();
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

