//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_api_key_request.g.dart';

/// Body for `POST /api/users/me/api-keys`.
///
/// Properties:
/// * [name]
/// * [scopes] - Scopes in `\"resource:action\"` format.
@BuiltValue()
abstract class CreateApiKeyRequest
    implements Built<CreateApiKeyRequest, CreateApiKeyRequestBuilder> {
  @BuiltValueField(wireName: r'name')
  String get name;

  /// Scopes in `\"resource:action\"` format.
  @BuiltValueField(wireName: r'scopes')
  BuiltList<String>? get scopes;

  CreateApiKeyRequest._();

  factory CreateApiKeyRequest([void updates(CreateApiKeyRequestBuilder b)]) =
      _$CreateApiKeyRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateApiKeyRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateApiKeyRequest> get serializer =>
      _$CreateApiKeyRequestSerializer();
}

class _$CreateApiKeyRequestSerializer
    implements PrimitiveSerializer<CreateApiKeyRequest> {
  @override
  final Iterable<Type> types = const [
    CreateApiKeyRequest,
    _$CreateApiKeyRequest
  ];

  @override
  final String wireName = r'CreateApiKeyRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateApiKeyRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    if (object.scopes != null) {
      yield r'scopes';
      yield serializers.serialize(
        object.scopes,
        specifiedType: const FullType.nullable(BuiltList, [FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateApiKeyRequest object, {
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
    required CreateApiKeyRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'scopes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType.nullable(BuiltList, [FullType(String)]),
          ) as BuiltList<String>?;
          if (valueDes == null) continue;
          result.scopes.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateApiKeyRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateApiKeyRequestBuilder();
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
