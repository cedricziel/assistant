//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_credential_request.g.dart';

/// Body of `PUT /api/integrations/{id}/credential`.
///
/// Properties:
/// * [headerName] - The header that carries the value. Only accepted when the integration has no header name yet, and then only from someone who manages its layer.
/// * [value] - The secret that fills the header template, e.g. the token after `Bearer `. Surrounding whitespace is dropped. It is stored encrypted and never returned.
@BuiltValue()
abstract class SetCredentialRequest
    implements Built<SetCredentialRequest, SetCredentialRequestBuilder> {
  /// The header that carries the value. Only accepted when the integration has no header name yet, and then only from someone who manages its layer.
  @BuiltValueField(wireName: r'header_name')
  String? get headerName;

  /// The secret that fills the header template, e.g. the token after `Bearer `. Surrounding whitespace is dropped. It is stored encrypted and never returned.
  @BuiltValueField(wireName: r'value')
  String get value;

  SetCredentialRequest._();

  factory SetCredentialRequest([void updates(SetCredentialRequestBuilder b)]) =
      _$SetCredentialRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetCredentialRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetCredentialRequest> get serializer =>
      _$SetCredentialRequestSerializer();
}

class _$SetCredentialRequestSerializer
    implements PrimitiveSerializer<SetCredentialRequest> {
  @override
  final Iterable<Type> types = const [
    SetCredentialRequest,
    _$SetCredentialRequest
  ];

  @override
  final String wireName = r'SetCredentialRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetCredentialRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.headerName != null) {
      yield r'header_name';
      yield serializers.serialize(
        object.headerName,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'value';
    yield serializers.serialize(
      object.value,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetCredentialRequest object, {
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
    required SetCredentialRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'header_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.headerName = valueDes;
          break;
        case r'value':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.value = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetCredentialRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetCredentialRequestBuilder();
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
