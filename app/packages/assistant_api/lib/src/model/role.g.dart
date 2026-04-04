// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'role.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const Role _$ROLE_UNSPECIFIED = const Role._('ROLE_UNSPECIFIED');
const Role _$ROLE_USER = const Role._('ROLE_USER');
const Role _$ROLE_AGENT = const Role._('ROLE_AGENT');
const Role _$unknownDefaultOpenApi = const Role._('unknownDefaultOpenApi');

Role _$valueOf(String name) {
  switch (name) {
    case 'ROLE_UNSPECIFIED':
      return _$ROLE_UNSPECIFIED;
    case 'ROLE_USER':
      return _$ROLE_USER;
    case 'ROLE_AGENT':
      return _$ROLE_AGENT;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<Role> _$values = BuiltSet<Role>(const <Role>[
  _$ROLE_UNSPECIFIED,
  _$ROLE_USER,
  _$ROLE_AGENT,
  _$unknownDefaultOpenApi,
]);

class _$RoleMeta {
  const _$RoleMeta();
  Role get ROLE_UNSPECIFIED => _$ROLE_UNSPECIFIED;
  Role get ROLE_USER => _$ROLE_USER;
  Role get ROLE_AGENT => _$ROLE_AGENT;
  Role get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  Role valueOf(String name) => _$valueOf(name);
  BuiltSet<Role> get values => _$values;
}

abstract class _$RoleMixin {
  // ignore: non_constant_identifier_names
  _$RoleMeta get Role => const _$RoleMeta();
}

Serializer<Role> _$roleSerializer = _$RoleSerializer();

class _$RoleSerializer implements PrimitiveSerializer<Role> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'ROLE_UNSPECIFIED': 'ROLE_UNSPECIFIED',
    'ROLE_USER': 'ROLE_USER',
    'ROLE_AGENT': 'ROLE_AGENT',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'ROLE_UNSPECIFIED': 'ROLE_UNSPECIFIED',
    'ROLE_USER': 'ROLE_USER',
    'ROLE_AGENT': 'ROLE_AGENT',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[Role];
  @override
  final String wireName = 'Role';

  @override
  Object serialize(Serializers serializers, Role object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  Role deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      Role.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
