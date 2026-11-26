// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rule_scope.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RuleScope _$own = const RuleScope._('own');
const RuleScope _$space = const RuleScope._('space');
const RuleScope _$unknownDefaultOpenApi =
    const RuleScope._('unknownDefaultOpenApi');

RuleScope _$valueOf(String name) {
  switch (name) {
    case 'own':
      return _$own;
    case 'space':
      return _$space;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<RuleScope> _$values = BuiltSet<RuleScope>(const <RuleScope>[
  _$own,
  _$space,
  _$unknownDefaultOpenApi,
]);

class _$RuleScopeMeta {
  const _$RuleScopeMeta();
  RuleScope get own => _$own;
  RuleScope get space => _$space;
  RuleScope get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  RuleScope valueOf(String name) => _$valueOf(name);
  BuiltSet<RuleScope> get values => _$values;
}

abstract class _$RuleScopeMixin {
  // ignore: non_constant_identifier_names
  _$RuleScopeMeta get RuleScope => const _$RuleScopeMeta();
}

Serializer<RuleScope> _$ruleScopeSerializer = _$RuleScopeSerializer();

class _$RuleScopeSerializer implements PrimitiveSerializer<RuleScope> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'own': 'own',
    'space': 'space',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'own': 'own',
    'space': 'space',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[RuleScope];
  @override
  final String wireName = 'RuleScope';

  @override
  Object serialize(Serializers serializers, RuleScope object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RuleScope deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RuleScope.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
