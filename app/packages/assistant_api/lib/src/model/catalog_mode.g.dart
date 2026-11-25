// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_mode.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const CatalogMode _$all = const CatalogMode._('all');
const CatalogMode _$allowlist = const CatalogMode._('allowlist');
const CatalogMode _$none = const CatalogMode._('none');
const CatalogMode _$unknownDefaultOpenApi =
    const CatalogMode._('unknownDefaultOpenApi');

CatalogMode _$valueOf(String name) {
  switch (name) {
    case 'all':
      return _$all;
    case 'allowlist':
      return _$allowlist;
    case 'none':
      return _$none;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<CatalogMode> _$values =
    BuiltSet<CatalogMode>(const <CatalogMode>[
  _$all,
  _$allowlist,
  _$none,
  _$unknownDefaultOpenApi,
]);

class _$CatalogModeMeta {
  const _$CatalogModeMeta();
  CatalogMode get all => _$all;
  CatalogMode get allowlist => _$allowlist;
  CatalogMode get none => _$none;
  CatalogMode get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  CatalogMode valueOf(String name) => _$valueOf(name);
  BuiltSet<CatalogMode> get values => _$values;
}

abstract class _$CatalogModeMixin {
  // ignore: non_constant_identifier_names
  _$CatalogModeMeta get CatalogMode => const _$CatalogModeMeta();
}

Serializer<CatalogMode> _$catalogModeSerializer = _$CatalogModeSerializer();

class _$CatalogModeSerializer implements PrimitiveSerializer<CatalogMode> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'all': 'all',
    'allowlist': 'allowlist',
    'none': 'none',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'all': 'all',
    'allowlist': 'allowlist',
    'none': 'none',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[CatalogMode];
  @override
  final String wireName = 'CatalogMode';

  @override
  Object serialize(Serializers serializers, CatalogMode object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  CatalogMode deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      CatalogMode.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
