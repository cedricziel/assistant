// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'space_access_mode.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const SpaceAccessMode _$allow = const SpaceAccessMode._('allow');
const SpaceAccessMode _$optOut = const SpaceAccessMode._('optOut');
const SpaceAccessMode _$unknownDefaultOpenApi =
    const SpaceAccessMode._('unknownDefaultOpenApi');

SpaceAccessMode _$valueOf(String name) {
  switch (name) {
    case 'allow':
      return _$allow;
    case 'optOut':
      return _$optOut;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<SpaceAccessMode> _$values =
    BuiltSet<SpaceAccessMode>(const <SpaceAccessMode>[
  _$allow,
  _$optOut,
  _$unknownDefaultOpenApi,
]);

class _$SpaceAccessModeMeta {
  const _$SpaceAccessModeMeta();
  SpaceAccessMode get allow => _$allow;
  SpaceAccessMode get optOut => _$optOut;
  SpaceAccessMode get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  SpaceAccessMode valueOf(String name) => _$valueOf(name);
  BuiltSet<SpaceAccessMode> get values => _$values;
}

abstract class _$SpaceAccessModeMixin {
  // ignore: non_constant_identifier_names
  _$SpaceAccessModeMeta get SpaceAccessMode => const _$SpaceAccessModeMeta();
}

Serializer<SpaceAccessMode> _$spaceAccessModeSerializer =
    _$SpaceAccessModeSerializer();

class _$SpaceAccessModeSerializer
    implements PrimitiveSerializer<SpaceAccessMode> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'allow': 'allow',
    'optOut': 'opt_out',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'allow': 'allow',
    'opt_out': 'optOut',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[SpaceAccessMode];
  @override
  final String wireName = 'SpaceAccessMode';

  @override
  Object serialize(Serializers serializers, SpaceAccessMode object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  SpaceAccessMode deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      SpaceAccessMode.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
