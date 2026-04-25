//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'publish_catalog_item_request.g.dart';

/// PublishCatalogItemRequest
///
/// Properties:
/// * [description]
/// * [name]
/// * [resourceType] - `\"skill\"`, `\"template\"`, or `\"interface\"`.
@BuiltValue()
abstract class PublishCatalogItemRequest
    implements
        Built<PublishCatalogItemRequest, PublishCatalogItemRequestBuilder> {
  @BuiltValueField(wireName: r'description')
  String? get description;

  @BuiltValueField(wireName: r'name')
  String get name;

  /// `\"skill\"`, `\"template\"`, or `\"interface\"`.
  @BuiltValueField(wireName: r'resource_type')
  String get resourceType;

  PublishCatalogItemRequest._();

  factory PublishCatalogItemRequest(
          [void updates(PublishCatalogItemRequestBuilder b)]) =
      _$PublishCatalogItemRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(PublishCatalogItemRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<PublishCatalogItemRequest> get serializer =>
      _$PublishCatalogItemRequestSerializer();
}

class _$PublishCatalogItemRequestSerializer
    implements PrimitiveSerializer<PublishCatalogItemRequest> {
  @override
  final Iterable<Type> types = const [
    PublishCatalogItemRequest,
    _$PublishCatalogItemRequest
  ];

  @override
  final String wireName = r'PublishCatalogItemRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    PublishCatalogItemRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.description != null) {
      yield r'description';
      yield serializers.serialize(
        object.description,
        specifiedType: const FullType(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'resource_type';
    yield serializers.serialize(
      object.resourceType,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    PublishCatalogItemRequest object, {
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
    required PublishCatalogItemRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'resource_type':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.resourceType = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  PublishCatalogItemRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = PublishCatalogItemRequestBuilder();
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
