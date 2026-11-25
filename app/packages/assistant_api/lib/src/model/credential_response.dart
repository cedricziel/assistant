//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'credential_response.g.dart';

/// Whether a credential is stored for the caller's subject. The value itself is never returned.
///
/// Properties:
/// * [hasCredential]
/// * [updatedAt] - When the credential was last set; absent when there is none.
@BuiltValue()
abstract class CredentialResponse
    implements Built<CredentialResponse, CredentialResponseBuilder> {
  @BuiltValueField(wireName: r'has_credential')
  bool get hasCredential;

  /// When the credential was last set; absent when there is none.
  @BuiltValueField(wireName: r'updated_at')
  DateTime? get updatedAt;

  CredentialResponse._();

  factory CredentialResponse([void updates(CredentialResponseBuilder b)]) =
      _$CredentialResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CredentialResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CredentialResponse> get serializer =>
      _$CredentialResponseSerializer();
}

class _$CredentialResponseSerializer
    implements PrimitiveSerializer<CredentialResponse> {
  @override
  final Iterable<Type> types = const [CredentialResponse, _$CredentialResponse];

  @override
  final String wireName = r'CredentialResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CredentialResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'has_credential';
    yield serializers.serialize(
      object.hasCredential,
      specifiedType: const FullType(bool),
    );
    if (object.updatedAt != null) {
      yield r'updated_at';
      yield serializers.serialize(
        object.updatedAt,
        specifiedType: const FullType.nullable(DateTime),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CredentialResponse object, {
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
    required CredentialResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'has_credential':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.hasCredential = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(DateTime),
          ) as DateTime?;
          if (valueDes == null) continue;
          result.updatedAt = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CredentialResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CredentialResponseBuilder();
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
