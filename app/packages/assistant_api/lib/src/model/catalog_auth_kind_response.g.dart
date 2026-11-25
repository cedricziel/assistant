// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_auth_kind_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const CatalogAuthKindResponse _$none = const CatalogAuthKindResponse._('none');
const CatalogAuthKindResponse _$header =
    const CatalogAuthKindResponse._('header');
const CatalogAuthKindResponse _$oauth =
    const CatalogAuthKindResponse._('oauth');
const CatalogAuthKindResponse _$mixed =
    const CatalogAuthKindResponse._('mixed');
const CatalogAuthKindResponse _$unknown =
    const CatalogAuthKindResponse._('unknown');
const CatalogAuthKindResponse _$unknownDefaultOpenApi =
    const CatalogAuthKindResponse._('unknownDefaultOpenApi');

CatalogAuthKindResponse _$valueOf(String name) {
  switch (name) {
    case 'none':
      return _$none;
    case 'header':
      return _$header;
    case 'oauth':
      return _$oauth;
    case 'mixed':
      return _$mixed;
    case 'unknown':
      return _$unknown;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<CatalogAuthKindResponse> _$values =
    BuiltSet<CatalogAuthKindResponse>(const <CatalogAuthKindResponse>[
  _$none,
  _$header,
  _$oauth,
  _$mixed,
  _$unknown,
  _$unknownDefaultOpenApi,
]);

class _$CatalogAuthKindResponseMeta {
  const _$CatalogAuthKindResponseMeta();
  CatalogAuthKindResponse get none => _$none;
  CatalogAuthKindResponse get header => _$header;
  CatalogAuthKindResponse get oauth => _$oauth;
  CatalogAuthKindResponse get mixed => _$mixed;
  CatalogAuthKindResponse get unknown => _$unknown;
  CatalogAuthKindResponse get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  CatalogAuthKindResponse valueOf(String name) => _$valueOf(name);
  BuiltSet<CatalogAuthKindResponse> get values => _$values;
}

abstract class _$CatalogAuthKindResponseMixin {
  // ignore: non_constant_identifier_names
  _$CatalogAuthKindResponseMeta get CatalogAuthKindResponse =>
      const _$CatalogAuthKindResponseMeta();
}

Serializer<CatalogAuthKindResponse> _$catalogAuthKindResponseSerializer =
    _$CatalogAuthKindResponseSerializer();

class _$CatalogAuthKindResponseSerializer
    implements PrimitiveSerializer<CatalogAuthKindResponse> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'none': 'none',
    'header': 'header',
    'oauth': 'oauth',
    'mixed': 'mixed',
    'unknown': 'unknown',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'none': 'none',
    'header': 'header',
    'oauth': 'oauth',
    'mixed': 'mixed',
    'unknown': 'unknown',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[CatalogAuthKindResponse];
  @override
  final String wireName = 'CatalogAuthKindResponse';

  @override
  Object serialize(Serializers serializers, CatalogAuthKindResponse object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  CatalogAuthKindResponse deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      CatalogAuthKindResponse.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
