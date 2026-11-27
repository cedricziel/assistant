// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'respond_to_interaction_request_one_of2.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RespondToInteractionRequestOneOf2KindEnum
    _$respondToInteractionRequestOneOf2KindEnum_surfaceAction =
    const RespondToInteractionRequestOneOf2KindEnum._('surfaceAction');
const RespondToInteractionRequestOneOf2KindEnum
    _$respondToInteractionRequestOneOf2KindEnum_unknownDefaultOpenApi =
    const RespondToInteractionRequestOneOf2KindEnum._('unknownDefaultOpenApi');

RespondToInteractionRequestOneOf2KindEnum
    _$respondToInteractionRequestOneOf2KindEnumValueOf(String name) {
  switch (name) {
    case 'surfaceAction':
      return _$respondToInteractionRequestOneOf2KindEnum_surfaceAction;
    case 'unknownDefaultOpenApi':
      return _$respondToInteractionRequestOneOf2KindEnum_unknownDefaultOpenApi;
    default:
      return _$respondToInteractionRequestOneOf2KindEnum_unknownDefaultOpenApi;
  }
}

final BuiltSet<RespondToInteractionRequestOneOf2KindEnum>
    _$respondToInteractionRequestOneOf2KindEnumValues = BuiltSet<
        RespondToInteractionRequestOneOf2KindEnum>(const <RespondToInteractionRequestOneOf2KindEnum>[
  _$respondToInteractionRequestOneOf2KindEnum_surfaceAction,
  _$respondToInteractionRequestOneOf2KindEnum_unknownDefaultOpenApi,
]);

Serializer<RespondToInteractionRequestOneOf2KindEnum>
    _$respondToInteractionRequestOneOf2KindEnumSerializer =
    _$RespondToInteractionRequestOneOf2KindEnumSerializer();

class _$RespondToInteractionRequestOneOf2KindEnumSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf2KindEnum> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'surfaceAction': 'surface_action',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'surface_action': 'surfaceAction',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[
    RespondToInteractionRequestOneOf2KindEnum
  ];
  @override
  final String wireName = 'RespondToInteractionRequestOneOf2KindEnum';

  @override
  Object serialize(Serializers serializers,
          RespondToInteractionRequestOneOf2KindEnum object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RespondToInteractionRequestOneOf2KindEnum deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RespondToInteractionRequestOneOf2KindEnum.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

class _$RespondToInteractionRequestOneOf2
    extends RespondToInteractionRequestOneOf2 {
  @override
  final JsonObject action;
  @override
  final JsonObject? dataModel;
  @override
  final RespondToInteractionRequestOneOf2KindEnum kind;

  factory _$RespondToInteractionRequestOneOf2(
          [void Function(RespondToInteractionRequestOneOf2Builder)? updates]) =>
      (RespondToInteractionRequestOneOf2Builder()..update(updates))._build();

  _$RespondToInteractionRequestOneOf2._(
      {required this.action, this.dataModel, required this.kind})
      : super._();
  @override
  RespondToInteractionRequestOneOf2 rebuild(
          void Function(RespondToInteractionRequestOneOf2Builder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RespondToInteractionRequestOneOf2Builder toBuilder() =>
      RespondToInteractionRequestOneOf2Builder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RespondToInteractionRequestOneOf2 &&
        action == other.action &&
        dataModel == other.dataModel &&
        kind == other.kind;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, action.hashCode);
    _$hash = $jc(_$hash, dataModel.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RespondToInteractionRequestOneOf2')
          ..add('action', action)
          ..add('dataModel', dataModel)
          ..add('kind', kind))
        .toString();
  }
}

class RespondToInteractionRequestOneOf2Builder
    implements
        Builder<RespondToInteractionRequestOneOf2,
            RespondToInteractionRequestOneOf2Builder> {
  _$RespondToInteractionRequestOneOf2? _$v;

  JsonObject? _action;
  JsonObject? get action => _$this._action;
  set action(JsonObject? action) => _$this._action = action;

  JsonObject? _dataModel;
  JsonObject? get dataModel => _$this._dataModel;
  set dataModel(JsonObject? dataModel) => _$this._dataModel = dataModel;

  RespondToInteractionRequestOneOf2KindEnum? _kind;
  RespondToInteractionRequestOneOf2KindEnum? get kind => _$this._kind;
  set kind(RespondToInteractionRequestOneOf2KindEnum? kind) =>
      _$this._kind = kind;

  RespondToInteractionRequestOneOf2Builder() {
    RespondToInteractionRequestOneOf2._defaults(this);
  }

  RespondToInteractionRequestOneOf2Builder get _$this {
    final $v = _$v;
    if ($v != null) {
      _action = $v.action;
      _dataModel = $v.dataModel;
      _kind = $v.kind;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RespondToInteractionRequestOneOf2 other) {
    _$v = other as _$RespondToInteractionRequestOneOf2;
  }

  @override
  void update(
      void Function(RespondToInteractionRequestOneOf2Builder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RespondToInteractionRequestOneOf2 build() => _build();

  _$RespondToInteractionRequestOneOf2 _build() {
    final _$result = _$v ??
        _$RespondToInteractionRequestOneOf2._(
          action: BuiltValueNullFieldError.checkNotNull(
              action, r'RespondToInteractionRequestOneOf2', 'action'),
          dataModel: dataModel,
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'RespondToInteractionRequestOneOf2', 'kind'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
