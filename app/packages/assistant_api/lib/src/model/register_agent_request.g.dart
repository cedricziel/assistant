// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_agent_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterAgentRequest extends RegisterAgentRequest {
  @override
  final JsonObject? card;
  @override
  final bool? setDefault;

  factory _$RegisterAgentRequest(
          [void Function(RegisterAgentRequestBuilder)? updates]) =>
      (RegisterAgentRequestBuilder()..update(updates))._build();

  _$RegisterAgentRequest._({this.card, this.setDefault}) : super._();
  @override
  RegisterAgentRequest rebuild(
          void Function(RegisterAgentRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RegisterAgentRequestBuilder toBuilder() =>
      RegisterAgentRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterAgentRequest &&
        card == other.card &&
        setDefault == other.setDefault;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, card.hashCode);
    _$hash = $jc(_$hash, setDefault.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegisterAgentRequest')
          ..add('card', card)
          ..add('setDefault', setDefault))
        .toString();
  }
}

class RegisterAgentRequestBuilder
    implements Builder<RegisterAgentRequest, RegisterAgentRequestBuilder> {
  _$RegisterAgentRequest? _$v;

  JsonObject? _card;
  JsonObject? get card => _$this._card;
  set card(JsonObject? card) => _$this._card = card;

  bool? _setDefault;
  bool? get setDefault => _$this._setDefault;
  set setDefault(bool? setDefault) => _$this._setDefault = setDefault;

  RegisterAgentRequestBuilder() {
    RegisterAgentRequest._defaults(this);
  }

  RegisterAgentRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _card = $v.card;
      _setDefault = $v.setDefault;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterAgentRequest other) {
    _$v = other as _$RegisterAgentRequest;
  }

  @override
  void update(void Function(RegisterAgentRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RegisterAgentRequest build() => _build();

  _$RegisterAgentRequest _build() {
    final _$result = _$v ??
        _$RegisterAgentRequest._(
          card: card,
          setDefault: setDefault,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
