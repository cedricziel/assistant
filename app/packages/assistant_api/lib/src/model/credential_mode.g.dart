// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credential_mode.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const CredentialMode _$shared = const CredentialMode._('shared');
const CredentialMode _$perUser = const CredentialMode._('perUser');
const CredentialMode _$unknownDefaultOpenApi =
    const CredentialMode._('unknownDefaultOpenApi');

CredentialMode _$valueOf(String name) {
  switch (name) {
    case 'shared':
      return _$shared;
    case 'perUser':
      return _$perUser;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<CredentialMode> _$values =
    BuiltSet<CredentialMode>(const <CredentialMode>[
  _$shared,
  _$perUser,
  _$unknownDefaultOpenApi,
]);

class _$CredentialModeMeta {
  const _$CredentialModeMeta();
  CredentialMode get shared => _$shared;
  CredentialMode get perUser => _$perUser;
  CredentialMode get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  CredentialMode valueOf(String name) => _$valueOf(name);
  BuiltSet<CredentialMode> get values => _$values;
}

abstract class _$CredentialModeMixin {
  // ignore: non_constant_identifier_names
  _$CredentialModeMeta get CredentialMode => const _$CredentialModeMeta();
}

Serializer<CredentialMode> _$credentialModeSerializer =
    _$CredentialModeSerializer();

class _$CredentialModeSerializer
    implements PrimitiveSerializer<CredentialMode> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'shared': 'shared',
    'perUser': 'per_user',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'shared': 'shared',
    'per_user': 'perUser',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[CredentialMode];
  @override
  final String wireName = 'CredentialMode';

  @override
  Object serialize(Serializers serializers, CredentialMode object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  CredentialMode deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      CredentialMode.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
