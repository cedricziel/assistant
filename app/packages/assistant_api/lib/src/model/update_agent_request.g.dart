// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_agent_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateAgentRequest extends UpdateAgentRequest {
  @override
  final JsonObject? card;

  factory _$UpdateAgentRequest(
          [void Function(UpdateAgentRequestBuilder)? updates]) =>
      (UpdateAgentRequestBuilder()..update(updates))._build();

  _$UpdateAgentRequest._({this.card}) : super._();
  @override
  UpdateAgentRequest rebuild(
          void Function(UpdateAgentRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateAgentRequestBuilder toBuilder() =>
      UpdateAgentRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateAgentRequest && card == other.card;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, card.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateAgentRequest')
          ..add('card', card))
        .toString();
  }
}

class UpdateAgentRequestBuilder
    implements Builder<UpdateAgentRequest, UpdateAgentRequestBuilder> {
  _$UpdateAgentRequest? _$v;

  JsonObject? _card;
  JsonObject? get card => _$this._card;
  set card(JsonObject? card) => _$this._card = card;

  UpdateAgentRequestBuilder() {
    UpdateAgentRequest._defaults(this);
  }

  UpdateAgentRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _card = $v.card;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateAgentRequest other) {
    _$v = other as _$UpdateAgentRequest;
  }

  @override
  void update(void Function(UpdateAgentRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateAgentRequest build() => _build();

  _$UpdateAgentRequest _build() {
    final _$result = _$v ??
        _$UpdateAgentRequest._(
          card: card,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
