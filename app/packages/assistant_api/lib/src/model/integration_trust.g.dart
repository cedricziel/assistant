// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_trust.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const IntegrationTrust _$confirm = const IntegrationTrust._('confirm');
const IntegrationTrust _$trust = const IntegrationTrust._('trust');
const IntegrationTrust _$unknownDefaultOpenApi =
    const IntegrationTrust._('unknownDefaultOpenApi');

IntegrationTrust _$valueOf(String name) {
  switch (name) {
    case 'confirm':
      return _$confirm;
    case 'trust':
      return _$trust;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<IntegrationTrust> _$values =
    BuiltSet<IntegrationTrust>(const <IntegrationTrust>[
  _$confirm,
  _$trust,
  _$unknownDefaultOpenApi,
]);

class _$IntegrationTrustMeta {
  const _$IntegrationTrustMeta();
  IntegrationTrust get confirm => _$confirm;
  IntegrationTrust get trust => _$trust;
  IntegrationTrust get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  IntegrationTrust valueOf(String name) => _$valueOf(name);
  BuiltSet<IntegrationTrust> get values => _$values;
}

abstract class _$IntegrationTrustMixin {
  // ignore: non_constant_identifier_names
  _$IntegrationTrustMeta get IntegrationTrust => const _$IntegrationTrustMeta();
}

Serializer<IntegrationTrust> _$integrationTrustSerializer =
    _$IntegrationTrustSerializer();

class _$IntegrationTrustSerializer
    implements PrimitiveSerializer<IntegrationTrust> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'confirm': 'confirm',
    'trust': 'trust',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'confirm': 'confirm',
    'trust': 'trust',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[IntegrationTrust];
  @override
  final String wireName = 'IntegrationTrust';

  @override
  Object serialize(Serializers serializers, IntegrationTrust object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  IntegrationTrust deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      IntegrationTrust.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
