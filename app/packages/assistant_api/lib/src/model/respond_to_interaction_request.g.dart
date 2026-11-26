// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'respond_to_interaction_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RespondToInteractionRequestKindEnum
    _$respondToInteractionRequestKindEnum_decline =
    const RespondToInteractionRequestKindEnum._('decline');
const RespondToInteractionRequestKindEnum
    _$respondToInteractionRequestKindEnum_unknownDefaultOpenApi =
    const RespondToInteractionRequestKindEnum._('unknownDefaultOpenApi');

RespondToInteractionRequestKindEnum
    _$respondToInteractionRequestKindEnumValueOf(String name) {
  switch (name) {
    case 'decline':
      return _$respondToInteractionRequestKindEnum_decline;
    case 'unknownDefaultOpenApi':
      return _$respondToInteractionRequestKindEnum_unknownDefaultOpenApi;
    default:
      return _$respondToInteractionRequestKindEnum_unknownDefaultOpenApi;
  }
}

final BuiltSet<RespondToInteractionRequestKindEnum>
    _$respondToInteractionRequestKindEnumValues = BuiltSet<
        RespondToInteractionRequestKindEnum>(const <RespondToInteractionRequestKindEnum>[
  _$respondToInteractionRequestKindEnum_decline,
  _$respondToInteractionRequestKindEnum_unknownDefaultOpenApi,
]);

Serializer<RespondToInteractionRequestKindEnum>
    _$respondToInteractionRequestKindEnumSerializer =
    _$RespondToInteractionRequestKindEnumSerializer();

class _$RespondToInteractionRequestKindEnumSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestKindEnum> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'decline': 'decline',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'decline': 'decline',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[
    RespondToInteractionRequestKindEnum
  ];
  @override
  final String wireName = 'RespondToInteractionRequestKindEnum';

  @override
  Object serialize(
          Serializers serializers, RespondToInteractionRequestKindEnum object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RespondToInteractionRequestKindEnum deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RespondToInteractionRequestKindEnum.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

class _$RespondToInteractionRequest extends RespondToInteractionRequest {
  @override
  final OneOf oneOf;

  factory _$RespondToInteractionRequest(
          [void Function(RespondToInteractionRequestBuilder)? updates]) =>
      (RespondToInteractionRequestBuilder()..update(updates))._build();

  _$RespondToInteractionRequest._({required this.oneOf}) : super._();
  @override
  RespondToInteractionRequest rebuild(
          void Function(RespondToInteractionRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RespondToInteractionRequestBuilder toBuilder() =>
      RespondToInteractionRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RespondToInteractionRequest && oneOf == other.oneOf;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, oneOf.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RespondToInteractionRequest')
          ..add('oneOf', oneOf))
        .toString();
  }
}

class RespondToInteractionRequestBuilder
    implements
        Builder<RespondToInteractionRequest,
            RespondToInteractionRequestBuilder> {
  _$RespondToInteractionRequest? _$v;

  OneOf? _oneOf;
  OneOf? get oneOf => _$this._oneOf;
  set oneOf(OneOf? oneOf) => _$this._oneOf = oneOf;

  RespondToInteractionRequestBuilder() {
    RespondToInteractionRequest._defaults(this);
  }

  RespondToInteractionRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _oneOf = $v.oneOf;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RespondToInteractionRequest other) {
    _$v = other as _$RespondToInteractionRequest;
  }

  @override
  void update(void Function(RespondToInteractionRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RespondToInteractionRequest build() => _build();

  _$RespondToInteractionRequest _build() {
    final _$result = _$v ??
        _$RespondToInteractionRequest._(
          oneOf: BuiltValueNullFieldError.checkNotNull(
              oneOf, r'RespondToInteractionRequest', 'oneOf'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
