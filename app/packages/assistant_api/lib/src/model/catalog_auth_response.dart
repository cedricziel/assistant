//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/catalog_auth_kind_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'catalog_auth_response.g.dart';

/// Authentication hints of a catalog entry.
///
/// Properties:
/// * [headerName] - Header to send, for `header` and `mixed` entries whose template parsed.
/// * [kind]
/// * [placeholder] - Name of the placeholder the user fills in, e.g. `api_key`.
/// * [valuePrefix] - Literal text before the secret in the header value, e.g. `Bearer `.
@BuiltValue()
abstract class CatalogAuthResponse
    implements Built<CatalogAuthResponse, CatalogAuthResponseBuilder> {
  /// Header to send, for `header` and `mixed` entries whose template parsed.
  @BuiltValueField(wireName: r'header_name')
  String? get headerName;

  @BuiltValueField(wireName: r'kind')
  CatalogAuthKindResponse get kind;
  // enum kindEnum {  none,  header,  oauth,  mixed,  unknown,  };

  /// Name of the placeholder the user fills in, e.g. `api_key`.
  @BuiltValueField(wireName: r'placeholder')
  String? get placeholder;

  /// Literal text before the secret in the header value, e.g. `Bearer `.
  @BuiltValueField(wireName: r'value_prefix')
  String? get valuePrefix;

  CatalogAuthResponse._();

  factory CatalogAuthResponse([void updates(CatalogAuthResponseBuilder b)]) =
      _$CatalogAuthResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CatalogAuthResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CatalogAuthResponse> get serializer =>
      _$CatalogAuthResponseSerializer();
}

class _$CatalogAuthResponseSerializer
    implements PrimitiveSerializer<CatalogAuthResponse> {
  @override
  final Iterable<Type> types = const [
    CatalogAuthResponse,
    _$CatalogAuthResponse
  ];

  @override
  final String wireName = r'CatalogAuthResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CatalogAuthResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.headerName != null) {
      yield r'header_name';
      yield serializers.serialize(
        object.headerName,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(CatalogAuthKindResponse),
    );
    if (object.placeholder != null) {
      yield r'placeholder';
      yield serializers.serialize(
        object.placeholder,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.valuePrefix != null) {
      yield r'value_prefix';
      yield serializers.serialize(
        object.valuePrefix,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CatalogAuthResponse object, {
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
    required CatalogAuthResponseBuilder result,
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
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(CatalogAuthKindResponse),
          ) as CatalogAuthKindResponse;
          result.kind = valueDes;
          break;
        case r'placeholder':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.placeholder = valueDes;
          break;
        case r'value_prefix':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.valuePrefix = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CatalogAuthResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CatalogAuthResponseBuilder();
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
