// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'interaction_outcome_view.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const InteractionOutcomeView _$resolved =
    const InteractionOutcomeView._('resolved');
const InteractionOutcomeView _$cancelled =
    const InteractionOutcomeView._('cancelled');
const InteractionOutcomeView _$expired =
    const InteractionOutcomeView._('expired');
const InteractionOutcomeView _$unknownDefaultOpenApi =
    const InteractionOutcomeView._('unknownDefaultOpenApi');

InteractionOutcomeView _$valueOf(String name) {
  switch (name) {
    case 'resolved':
      return _$resolved;
    case 'cancelled':
      return _$cancelled;
    case 'expired':
      return _$expired;
    case 'unknownDefaultOpenApi':
      return _$unknownDefaultOpenApi;
    default:
      return _$unknownDefaultOpenApi;
  }
}

final BuiltSet<InteractionOutcomeView> _$values =
    BuiltSet<InteractionOutcomeView>(const <InteractionOutcomeView>[
  _$resolved,
  _$cancelled,
  _$expired,
  _$unknownDefaultOpenApi,
]);

class _$InteractionOutcomeViewMeta {
  const _$InteractionOutcomeViewMeta();
  InteractionOutcomeView get resolved => _$resolved;
  InteractionOutcomeView get cancelled => _$cancelled;
  InteractionOutcomeView get expired => _$expired;
  InteractionOutcomeView get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  InteractionOutcomeView valueOf(String name) => _$valueOf(name);
  BuiltSet<InteractionOutcomeView> get values => _$values;
}

abstract class _$InteractionOutcomeViewMixin {
  // ignore: non_constant_identifier_names
  _$InteractionOutcomeViewMeta get InteractionOutcomeView =>
      const _$InteractionOutcomeViewMeta();
}

Serializer<InteractionOutcomeView> _$interactionOutcomeViewSerializer =
    _$InteractionOutcomeViewSerializer();

class _$InteractionOutcomeViewSerializer
    implements PrimitiveSerializer<InteractionOutcomeView> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'resolved': 'resolved',
    'cancelled': 'cancelled',
    'expired': 'expired',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'resolved': 'resolved',
    'cancelled': 'cancelled',
    'expired': 'expired',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[InteractionOutcomeView];
  @override
  final String wireName = 'InteractionOutcomeView';

  @override
  Object serialize(Serializers serializers, InteractionOutcomeView object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  InteractionOutcomeView deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      InteractionOutcomeView.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
