// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'turn_state.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const TurnState _$running = const TurnState._('running');
const TurnState _$completed = const TurnState._('completed');
const TurnState _$errored = const TurnState._('errored');
const TurnState _$unknown = const TurnState._('unknown');
const TurnState _$unknownDefaultOpenApi =
    const TurnState._('unknownDefaultOpenApi');

TurnState _$valueOf(String name) {
  switch (name) {
    case 'running':
      return _$running;
    case 'completed':
      return _$completed;
    case 'errored':
      return _$errored;
    case 'unknown':
      return _$unknown;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<TurnState> _$values = BuiltSet<TurnState>(const <TurnState>[
  _$running,
  _$completed,
  _$errored,
  _$unknown,
  _$unknownDefaultOpenApi,
]);

class _$TurnStateMeta {
  const _$TurnStateMeta();
  TurnState get running => _$running;
  TurnState get completed => _$completed;
  TurnState get errored => _$errored;
  TurnState get unknown => _$unknown;
  TurnState get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  TurnState valueOf(String name) => _$valueOf(name);
  BuiltSet<TurnState> get values => _$values;
}

abstract class _$TurnStateMixin {
  // ignore: non_constant_identifier_names
  _$TurnStateMeta get TurnState => const _$TurnStateMeta();
}

Serializer<TurnState> _$turnStateSerializer = _$TurnStateSerializer();

class _$TurnStateSerializer implements PrimitiveSerializer<TurnState> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'running': 'running',
    'completed': 'completed',
    'errored': 'errored',
    'unknown': 'unknown',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'running': 'running',
    'completed': 'completed',
    'errored': 'errored',
    'unknown': 'unknown',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[TurnState];
  @override
  final String wireName = 'TurnState';

  @override
  Object serialize(Serializers serializers, TurnState object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  TurnState deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      TurnState.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
