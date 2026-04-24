//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_api_key_response.g.dart';

/// Response for `POST /api/users/me/api-keys` — includes plaintext key (shown once).
///
/// Properties:
/// * [id]
/// * [keyPrefix]
/// * [name]
/// * [plaintext] - The full API key — shown only on creation.
/// * [scopes]
@BuiltValue()
abstract class CreateApiKeyResponse
    implements Built<CreateApiKeyResponse, CreateApiKeyResponseBuilder> {
  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'key_prefix')
  String get keyPrefix;

  @BuiltValueField(wireName: r'name')
  String get name;

  /// The full API key — shown only on creation.
  @BuiltValueField(wireName: r'plaintext')
  String get plaintext;

  @BuiltValueField(wireName: r'scopes')
  BuiltList<String> get scopes;

  CreateApiKeyResponse._();

  factory CreateApiKeyResponse([void updates(CreateApiKeyResponseBuilder b)]) =
      _$CreateApiKeyResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateApiKeyResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateApiKeyResponse> get serializer =>
      _$CreateApiKeyResponseSerializer();
}

class _$CreateApiKeyResponseSerializer
    implements PrimitiveSerializer<CreateApiKeyResponse> {
  @override
  final Iterable<Type> types = const [
    CreateApiKeyResponse,
    _$CreateApiKeyResponse
  ];

  @override
  final String wireName = r'CreateApiKeyResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateApiKeyResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'key_prefix';
    yield serializers.serialize(
      object.keyPrefix,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'plaintext';
    yield serializers.serialize(
      object.plaintext,
      specifiedType: const FullType(String),
    );
    yield r'scopes';
    yield serializers.serialize(
      object.scopes,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateApiKeyResponse object, {
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
    required CreateApiKeyResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'key_prefix':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.keyPrefix = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'plaintext':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.plaintext = valueDes;
          break;
        case r'scopes':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
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
  CreateApiKeyResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateApiKeyResponseBuilder();
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
