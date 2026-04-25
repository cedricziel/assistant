//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_webhook_request.g.dart';

/// Body for `POST /api/webhooks`.
///
/// Properties:
/// * [active] - Defaults to `true` when omitted.
/// * [eventTypes]
/// * [name]
/// * [url]
@BuiltValue()
abstract class CreateWebhookRequest
    implements Built<CreateWebhookRequest, CreateWebhookRequestBuilder> {
  /// Defaults to `true` when omitted.
  @BuiltValueField(wireName: r'active')
  bool? get active;

  @BuiltValueField(wireName: r'event_types')
  BuiltList<String> get eventTypes;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'url')
  String get url;

  CreateWebhookRequest._();

  factory CreateWebhookRequest([void updates(CreateWebhookRequestBuilder b)]) =
      _$CreateWebhookRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateWebhookRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateWebhookRequest> get serializer =>
      _$CreateWebhookRequestSerializer();
}

class _$CreateWebhookRequestSerializer
    implements PrimitiveSerializer<CreateWebhookRequest> {
  @override
  final Iterable<Type> types = const [
    CreateWebhookRequest,
    _$CreateWebhookRequest
  ];

  @override
  final String wireName = r'CreateWebhookRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateWebhookRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.active != null) {
      yield r'active';
      yield serializers.serialize(
        object.active,
        specifiedType: const FullType.nullable(bool),
      );
    }
    yield r'event_types';
    yield serializers.serialize(
      object.eventTypes,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'url';
    yield serializers.serialize(
      object.url,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateWebhookRequest object, {
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
    required CreateWebhookRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'active':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.active = valueDes;
          break;
        case r'event_types':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.eventTypes.replace(valueDes);
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
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
  CreateWebhookRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateWebhookRequestBuilder();
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
