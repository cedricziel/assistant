// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_auth_kind.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const IntegrationAuthKind _$none = const IntegrationAuthKind._('none');
const IntegrationAuthKind _$header = const IntegrationAuthKind._('header');
const IntegrationAuthKind _$oauth = const IntegrationAuthKind._('oauth');
const IntegrationAuthKind _$mixed = const IntegrationAuthKind._('mixed');
const IntegrationAuthKind _$unknownDefaultOpenApi =
    const IntegrationAuthKind._('unknownDefaultOpenApi');

IntegrationAuthKind _$valueOf(String name) {
  switch (name) {
    case 'none':
      return _$none;
    case 'header':
      return _$header;
    case 'oauth':
      return _$oauth;
    case 'mixed':
      return _$mixed;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<IntegrationAuthKind> _$values =
    BuiltSet<IntegrationAuthKind>(const <IntegrationAuthKind>[
  _$none,
  _$header,
  _$oauth,
  _$mixed,
  _$unknownDefaultOpenApi,
]);

class _$IntegrationAuthKindMeta {
  const _$IntegrationAuthKindMeta();
  IntegrationAuthKind get none => _$none;
  IntegrationAuthKind get header => _$header;
  IntegrationAuthKind get oauth => _$oauth;
  IntegrationAuthKind get mixed => _$mixed;
  IntegrationAuthKind get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  IntegrationAuthKind valueOf(String name) => _$valueOf(name);
  BuiltSet<IntegrationAuthKind> get values => _$values;
}

abstract class _$IntegrationAuthKindMixin {
  // ignore: non_constant_identifier_names
  _$IntegrationAuthKindMeta get IntegrationAuthKind =>
      const _$IntegrationAuthKindMeta();
}

Serializer<IntegrationAuthKind> _$integrationAuthKindSerializer =
    _$IntegrationAuthKindSerializer();

class _$IntegrationAuthKindSerializer
    implements PrimitiveSerializer<IntegrationAuthKind> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'none': 'none',
    'header': 'header',
    'oauth': 'oauth',
    'mixed': 'mixed',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'none': 'none',
    'header': 'header',
    'oauth': 'oauth',
    'mixed': 'mixed',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[IntegrationAuthKind];
  @override
  final String wireName = 'IntegrationAuthKind';

  @override
  Object serialize(Serializers serializers, IntegrationAuthKind object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  IntegrationAuthKind deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      IntegrationAuthKind.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
