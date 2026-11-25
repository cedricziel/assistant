// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_layer.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const IntegrationLayer _$org = const IntegrationLayer._('org');
const IntegrationLayer _$space = const IntegrationLayer._('space');
const IntegrationLayer _$user = const IntegrationLayer._('user');
const IntegrationLayer _$unknownDefaultOpenApi =
    const IntegrationLayer._('unknownDefaultOpenApi');

IntegrationLayer _$valueOf(String name) {
  switch (name) {
    case 'org':
      return _$org;
    case 'space':
      return _$space;
    case 'user':
      return _$user;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<IntegrationLayer> _$values =
    BuiltSet<IntegrationLayer>(const <IntegrationLayer>[
  _$org,
  _$space,
  _$user,
  _$unknownDefaultOpenApi,
]);

class _$IntegrationLayerMeta {
  const _$IntegrationLayerMeta();
  IntegrationLayer get org => _$org;
  IntegrationLayer get space => _$space;
  IntegrationLayer get user => _$user;
  IntegrationLayer get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  IntegrationLayer valueOf(String name) => _$valueOf(name);
  BuiltSet<IntegrationLayer> get values => _$values;
}

abstract class _$IntegrationLayerMixin {
  // ignore: non_constant_identifier_names
  _$IntegrationLayerMeta get IntegrationLayer => const _$IntegrationLayerMeta();
}

Serializer<IntegrationLayer> _$integrationLayerSerializer =
    _$IntegrationLayerSerializer();

class _$IntegrationLayerSerializer
    implements PrimitiveSerializer<IntegrationLayer> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'org': 'org',
    'space': 'space',
    'user': 'user',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'org': 'org',
    'space': 'space',
    'user': 'user',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[IntegrationLayer];
  @override
  final String wireName = 'IntegrationLayer';

  @override
  Object serialize(Serializers serializers, IntegrationLayer object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  IntegrationLayer deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      IntegrationLayer.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
