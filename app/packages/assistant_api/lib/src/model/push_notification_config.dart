//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/authentication_info.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'push_notification_config.g.dart';

/// Configuration for setting up push notifications for task updates.
///
/// Properties:
/// * [authentication] - Authentication information required to send the notification.
/// * [id] - A unique identifier (UUID) for this push notification configuration.
/// * [token] - A token unique for this task or session.
/// * [url] - The URL where the notification should be sent.
@BuiltValue()
abstract class PushNotificationConfig implements Built<PushNotificationConfig, PushNotificationConfigBuilder> {
  /// Authentication information required to send the notification.
  @BuiltValueField(wireName: r'authentication')
  AuthenticationInfo? get authentication;

  /// A unique identifier (UUID) for this push notification configuration.
  @BuiltValueField(wireName: r'id')
  String? get id;

  /// A token unique for this task or session.
  @BuiltValueField(wireName: r'token')
  String? get token;

  /// The URL where the notification should be sent.
  @BuiltValueField(wireName: r'url')
  String get url;

  PushNotificationConfig._();

  factory PushNotificationConfig([void updates(PushNotificationConfigBuilder b)]) = _$PushNotificationConfig;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PushNotificationConfigBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PushNotificationConfig> get serializer => _$PushNotificationConfigSerializer();
}

class _$PushNotificationConfigSerializer implements PrimitiveSerializer<PushNotificationConfig> {
  @override
  final Iterable<Type> types = const [PushNotificationConfig, _$PushNotificationConfig];

  @override
  final String wireName = r'PushNotificationConfig';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PushNotificationConfig object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.authentication != null) {
      yield r'authentication';
      yield serializers.serialize(
        object.authentication,
        specifiedType: const FullType.nullable(AuthenticationInfo),
      );
    }
    if (object.id != null) {
      yield r'id';
      yield serializers.serialize(
        object.id,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.token != null) {
      yield r'token';
      yield serializers.serialize(
        object.token,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'url';
    yield serializers.serialize(
      object.url,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PushNotificationConfig object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required PushNotificationConfigBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'authentication':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(AuthenticationInfo),
          ) as AuthenticationInfo?;
          if (valueDes == null) continue;
          result.authentication.replace(valueDes);
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.id = valueDes;
          break;
        case r'token':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.token = valueDes;
          break;
        case r'url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.url = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PushNotificationConfig deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PushNotificationConfigBuilder();
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

