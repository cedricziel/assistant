// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'respond_to_interaction_request_one_of.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RespondToInteractionRequestOneOfKindEnum
    _$respondToInteractionRequestOneOfKindEnum_toolApproval =
    const RespondToInteractionRequestOneOfKindEnum._('toolApproval');
const RespondToInteractionRequestOneOfKindEnum
    _$respondToInteractionRequestOneOfKindEnum_unknownDefaultOpenApi =
    const RespondToInteractionRequestOneOfKindEnum._('unknownDefaultOpenApi');

RespondToInteractionRequestOneOfKindEnum
    _$respondToInteractionRequestOneOfKindEnumValueOf(String name) {
  switch (name) {
    case 'toolApproval':
      return _$respondToInteractionRequestOneOfKindEnum_toolApproval;
    case 'unknownDefaultOpenApi':
      return _$respondToInteractionRequestOneOfKindEnum_unknownDefaultOpenApi;
    default:
      return _$respondToInteractionRequestOneOfKindEnum_unknownDefaultOpenApi;
  }
}

final BuiltSet<RespondToInteractionRequestOneOfKindEnum>
    _$respondToInteractionRequestOneOfKindEnumValues = BuiltSet<
        RespondToInteractionRequestOneOfKindEnum>(const <RespondToInteractionRequestOneOfKindEnum>[
  _$respondToInteractionRequestOneOfKindEnum_toolApproval,
  _$respondToInteractionRequestOneOfKindEnum_unknownDefaultOpenApi,
]);

Serializer<RespondToInteractionRequestOneOfKindEnum>
    _$respondToInteractionRequestOneOfKindEnumSerializer =
    _$RespondToInteractionRequestOneOfKindEnumSerializer();

class _$RespondToInteractionRequestOneOfKindEnumSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOfKindEnum> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'toolApproval': 'tool_approval',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'tool_approval': 'toolApproval',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[
    RespondToInteractionRequestOneOfKindEnum
  ];
  @override
  final String wireName = 'RespondToInteractionRequestOneOfKindEnum';

  @override
  Object serialize(Serializers serializers,
          RespondToInteractionRequestOneOfKindEnum object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RespondToInteractionRequestOneOfKindEnum deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RespondToInteractionRequestOneOfKindEnum.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

class _$RespondToInteractionRequestOneOf
    extends RespondToInteractionRequestOneOf {
  @override
  final ToolDecision decision;
  @override
  final RespondToInteractionRequestOneOfKindEnum kind;

  factory _$RespondToInteractionRequestOneOf(
          [void Function(RespondToInteractionRequestOneOfBuilder)? updates]) =>
      (RespondToInteractionRequestOneOfBuilder()..update(updates))._build();

  _$RespondToInteractionRequestOneOf._(
      {required this.decision, required this.kind})
      : super._();
  @override
  RespondToInteractionRequestOneOf rebuild(
          void Function(RespondToInteractionRequestOneOfBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RespondToInteractionRequestOneOfBuilder toBuilder() =>
      RespondToInteractionRequestOneOfBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RespondToInteractionRequestOneOf &&
        decision == other.decision &&
        kind == other.kind;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, decision.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RespondToInteractionRequestOneOf')
          ..add('decision', decision)
          ..add('kind', kind))
        .toString();
  }
}

class RespondToInteractionRequestOneOfBuilder
    implements
        Builder<RespondToInteractionRequestOneOf,
            RespondToInteractionRequestOneOfBuilder> {
  _$RespondToInteractionRequestOneOf? _$v;

  ToolDecision? _decision;
  ToolDecision? get decision => _$this._decision;
  set decision(ToolDecision? decision) => _$this._decision = decision;

  RespondToInteractionRequestOneOfKindEnum? _kind;
  RespondToInteractionRequestOneOfKindEnum? get kind => _$this._kind;
  set kind(RespondToInteractionRequestOneOfKindEnum? kind) =>
      _$this._kind = kind;

  RespondToInteractionRequestOneOfBuilder() {
    RespondToInteractionRequestOneOf._defaults(this);
  }

  RespondToInteractionRequestOneOfBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _decision = $v.decision;
      _kind = $v.kind;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RespondToInteractionRequestOneOf other) {
    _$v = other as _$RespondToInteractionRequestOneOf;
  }

  @override
  void update(void Function(RespondToInteractionRequestOneOfBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RespondToInteractionRequestOneOf build() => _build();

  _$RespondToInteractionRequestOneOf _build() {
    final _$result = _$v ??
        _$RespondToInteractionRequestOneOf._(
          decision: BuiltValueNullFieldError.checkNotNull(
              decision, r'RespondToInteractionRequestOneOf', 'decision'),
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'RespondToInteractionRequestOneOf', 'kind'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
