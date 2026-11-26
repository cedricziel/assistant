// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'interaction_status_view.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const InteractionStatusView _$pending =
    const InteractionStatusView._('pending');
const InteractionStatusView _$resolved =
    const InteractionStatusView._('resolved');
const InteractionStatusView _$cancelled =
    const InteractionStatusView._('cancelled');
const InteractionStatusView _$expired =
    const InteractionStatusView._('expired');
const InteractionStatusView _$unknownDefaultOpenApi =
    const InteractionStatusView._('unknownDefaultOpenApi');

InteractionStatusView _$valueOf(String name) {
  switch (name) {
    case 'pending':
      return _$pending;
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

final BuiltSet<InteractionStatusView> _$values =
    BuiltSet<InteractionStatusView>(const <InteractionStatusView>[
  _$pending,
  _$resolved,
  _$cancelled,
  _$expired,
  _$unknownDefaultOpenApi,
]);

class _$InteractionStatusViewMeta {
  const _$InteractionStatusViewMeta();
  InteractionStatusView get pending => _$pending;
  InteractionStatusView get resolved => _$resolved;
  InteractionStatusView get cancelled => _$cancelled;
  InteractionStatusView get expired => _$expired;
  InteractionStatusView get unknownDefaultOpenApi => _$unknownDefaultOpenApi;
  InteractionStatusView valueOf(String name) => _$valueOf(name);
  BuiltSet<InteractionStatusView> get values => _$values;
}

abstract class _$InteractionStatusViewMixin {
  // ignore: non_constant_identifier_names
  _$InteractionStatusViewMeta get InteractionStatusView =>
      const _$InteractionStatusViewMeta();
}

Serializer<InteractionStatusView> _$interactionStatusViewSerializer =
    _$InteractionStatusViewSerializer();

class _$InteractionStatusViewSerializer
    implements PrimitiveSerializer<InteractionStatusView> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'pending': 'pending',
    'resolved': 'resolved',
    'cancelled': 'cancelled',
    'expired': 'expired',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'pending': 'pending',
    'resolved': 'resolved',
    'cancelled': 'cancelled',
    'expired': 'expired',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[InteractionStatusView];
  @override
  final String wireName = 'InteractionStatusView';

  @override
  Object serialize(Serializers serializers, InteractionStatusView object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  InteractionStatusView deserialize(Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      InteractionStatusView.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
