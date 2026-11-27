// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'respond_to_interaction_request_one_of3.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RespondToInteractionRequestOneOf3KindEnum
    _$respondToInteractionRequestOneOf3KindEnum_decline =
    const RespondToInteractionRequestOneOf3KindEnum._('decline');
const RespondToInteractionRequestOneOf3KindEnum
    _$respondToInteractionRequestOneOf3KindEnum_unknownDefaultOpenApi =
    const RespondToInteractionRequestOneOf3KindEnum._('unknownDefaultOpenApi');

RespondToInteractionRequestOneOf3KindEnum
    _$respondToInteractionRequestOneOf3KindEnumValueOf(String name) {
  switch (name) {
    case 'decline':
      return _$respondToInteractionRequestOneOf3KindEnum_decline;
    case 'unknownDefaultOpenApi':
      return _$respondToInteractionRequestOneOf3KindEnum_unknownDefaultOpenApi;
    default:
      return _$respondToInteractionRequestOneOf3KindEnum_unknownDefaultOpenApi;
  }
}

final BuiltSet<RespondToInteractionRequestOneOf3KindEnum>
    _$respondToInteractionRequestOneOf3KindEnumValues = BuiltSet<
        RespondToInteractionRequestOneOf3KindEnum>(const <RespondToInteractionRequestOneOf3KindEnum>[
  _$respondToInteractionRequestOneOf3KindEnum_decline,
  _$respondToInteractionRequestOneOf3KindEnum_unknownDefaultOpenApi,
]);

Serializer<RespondToInteractionRequestOneOf3KindEnum>
    _$respondToInteractionRequestOneOf3KindEnumSerializer =
    _$RespondToInteractionRequestOneOf3KindEnumSerializer();

class _$RespondToInteractionRequestOneOf3KindEnumSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf3KindEnum> {
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
    RespondToInteractionRequestOneOf3KindEnum
  ];
  @override
  final String wireName = 'RespondToInteractionRequestOneOf3KindEnum';

  @override
  Object serialize(Serializers serializers,
          RespondToInteractionRequestOneOf3KindEnum object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RespondToInteractionRequestOneOf3KindEnum deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RespondToInteractionRequestOneOf3KindEnum.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

class _$RespondToInteractionRequestOneOf3
    extends RespondToInteractionRequestOneOf3 {
  @override
  final RespondToInteractionRequestOneOf3KindEnum kind;

  factory _$RespondToInteractionRequestOneOf3(
          [void Function(RespondToInteractionRequestOneOf3Builder)? updates]) =>
      (RespondToInteractionRequestOneOf3Builder()..update(updates))._build();

  _$RespondToInteractionRequestOneOf3._({required this.kind}) : super._();
  @override
  RespondToInteractionRequestOneOf3 rebuild(
          void Function(RespondToInteractionRequestOneOf3Builder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RespondToInteractionRequestOneOf3Builder toBuilder() =>
      RespondToInteractionRequestOneOf3Builder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RespondToInteractionRequestOneOf3 && kind == other.kind;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RespondToInteractionRequestOneOf3')
          ..add('kind', kind))
        .toString();
  }
}

class RespondToInteractionRequestOneOf3Builder
    implements
        Builder<RespondToInteractionRequestOneOf3,
            RespondToInteractionRequestOneOf3Builder> {
  _$RespondToInteractionRequestOneOf3? _$v;

  RespondToInteractionRequestOneOf3KindEnum? _kind;
  RespondToInteractionRequestOneOf3KindEnum? get kind => _$this._kind;
  set kind(RespondToInteractionRequestOneOf3KindEnum? kind) =>
      _$this._kind = kind;

  RespondToInteractionRequestOneOf3Builder() {
    RespondToInteractionRequestOneOf3._defaults(this);
  }

  RespondToInteractionRequestOneOf3Builder get _$this {
    final $v = _$v;
    if ($v != null) {
      _kind = $v.kind;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RespondToInteractionRequestOneOf3 other) {
    _$v = other as _$RespondToInteractionRequestOneOf3;
  }

  @override
  void update(
      void Function(RespondToInteractionRequestOneOf3Builder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RespondToInteractionRequestOneOf3 build() => _build();

  _$RespondToInteractionRequestOneOf3 _build() {
    final _$result = _$v ??
        _$RespondToInteractionRequestOneOf3._(
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'RespondToInteractionRequestOneOf3', 'kind'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
