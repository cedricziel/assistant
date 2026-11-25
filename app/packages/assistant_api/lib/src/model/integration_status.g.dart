// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'integration_status.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const IntegrationStatus _$ready = const IntegrationStatus._('ready');
const IntegrationStatus _$needsConnection =
    const IntegrationStatus._('needsConnection');
const IntegrationStatus _$authExpired =
    const IntegrationStatus._('authExpired');
const IntegrationStatus _$unreachable =
    const IntegrationStatus._('unreachable');
const IntegrationStatus _$blockedByPolicy =
    const IntegrationStatus._('blockedByPolicy');
const IntegrationStatus _$disabled = const IntegrationStatus._('disabled');
const IntegrationStatus _$unknownDefaultOpenApi =
    const IntegrationStatus._('unknownDefaultOpenApi');

IntegrationStatus _$valueOf(String name) {
  switch (name) {
    case 'ready':
      return _$ready;
    case 'needsConnection':
      return _$needsConnection;
    case 'authExpired':
      return _$authExpired;
    case 'unreachable':
      return _$unreachable;
    case 'blockedByPolicy':
      return _$blockedByPolicy;
    case 'disabled':
      return _$disabled;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<IntegrationStatus> _$values =
    BuiltSet<IntegrationStatus>(const <IntegrationStatus>[
  _$ready,
  _$needsConnection,
  _$authExpired,
  _$unreachable,
  _$blockedByPolicy,
  _$disabled,
  _$unknownDefaultOpenApi,
]);

class _$IntegrationStatusMeta {
  const _$IntegrationStatusMeta();
  IntegrationStatus get ready => _$ready;
  IntegrationStatus get needsConnection => _$needsConnection;
  IntegrationStatus get authExpired => _$authExpired;
  IntegrationStatus get unreachable => _$unreachable;
  IntegrationStatus get blockedByPolicy => _$blockedByPolicy;
  IntegrationStatus get disabled => _$disabled;
  IntegrationStatus get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  IntegrationStatus valueOf(String name) => _$valueOf(name);
  BuiltSet<IntegrationStatus> get values => _$values;
}

abstract class _$IntegrationStatusMixin {
  // ignore: non_constant_identifier_names
  _$IntegrationStatusMeta get IntegrationStatus =>
      const _$IntegrationStatusMeta();
}

Serializer<IntegrationStatus> _$integrationStatusSerializer =
    _$IntegrationStatusSerializer();

class _$IntegrationStatusSerializer
    implements PrimitiveSerializer<IntegrationStatus> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'ready': 'ready',
    'needsConnection': 'needs_connection',
    'authExpired': 'auth_expired',
    'unreachable': 'unreachable',
    'blockedByPolicy': 'blocked_by_policy',
    'disabled': 'disabled',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'ready': 'ready',
    'needs_connection': 'needsConnection',
    'auth_expired': 'authExpired',
    'unreachable': 'unreachable',
    'blocked_by_policy': 'blockedByPolicy',
    'disabled': 'disabled',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[IntegrationStatus];
  @override
  final String wireName = 'IntegrationStatus';

  @override
  Object serialize(Serializers serializers, IntegrationStatus object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  IntegrationStatus deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      IntegrationStatus.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
