// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tool_decision.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const ToolDecision _$allowOnce = const ToolDecision._('allowOnce');
const ToolDecision _$allowAlways = const ToolDecision._('allowAlways');
const ToolDecision _$rejectOnce = const ToolDecision._('rejectOnce');
const ToolDecision _$rejectAlways = const ToolDecision._('rejectAlways');
const ToolDecision _$unknownDefaultOpenApi =
    const ToolDecision._('unknownDefaultOpenApi');

ToolDecision _$valueOf(String name) {
  switch (name) {
    case 'allowOnce':
      return _$allowOnce;
    case 'allowAlways':
      return _$allowAlways;
    case 'rejectOnce':
      return _$rejectOnce;
    case 'rejectAlways':
      return _$rejectAlways;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<ToolDecision> _$values =
    BuiltSet<ToolDecision>(const <ToolDecision>[
  _$allowOnce,
  _$allowAlways,
  _$rejectOnce,
  _$rejectAlways,
  _$unknownDefaultOpenApi,
]);

class _$ToolDecisionMeta {
  const _$ToolDecisionMeta();
  ToolDecision get allowOnce => _$allowOnce;
  ToolDecision get allowAlways => _$allowAlways;
  ToolDecision get rejectOnce => _$rejectOnce;
  ToolDecision get rejectAlways => _$rejectAlways;
  ToolDecision get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  ToolDecision valueOf(String name) => _$valueOf(name);
  BuiltSet<ToolDecision> get values => _$values;
}

abstract class _$ToolDecisionMixin {
  // ignore: non_constant_identifier_names
  _$ToolDecisionMeta get ToolDecision => const _$ToolDecisionMeta();
}

Serializer<ToolDecision> _$toolDecisionSerializer = _$ToolDecisionSerializer();

class _$ToolDecisionSerializer implements PrimitiveSerializer<ToolDecision> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'allowOnce': 'allow_once',
    'allowAlways': 'allow_always',
    'rejectOnce': 'reject_once',
    'rejectAlways': 'reject_always',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'allow_once': 'allowOnce',
    'allow_always': 'allowAlways',
    'reject_once': 'rejectOnce',
    'reject_always': 'rejectAlways',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[ToolDecision];
  @override
  final String wireName = 'ToolDecision';

  @override
  Object serialize(Serializers serializers, ToolDecision object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  ToolDecision deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      ToolDecision.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
