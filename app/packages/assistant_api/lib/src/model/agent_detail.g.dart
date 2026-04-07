// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentDetail extends AgentDetail {
  @override
  final JsonObject? card;
  @override
  final String id;
  @override
  final bool isDefault;

  factory _$AgentDetail([void Function(AgentDetailBuilder)? updates]) =>
      (AgentDetailBuilder()..update(updates))._build();

  _$AgentDetail._({this.card, required this.id, required this.isDefault})
      : super._();
  @override
  AgentDetail rebuild(void Function(AgentDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentDetailBuilder toBuilder() => AgentDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentDetail &&
        card == other.card &&
        id == other.id &&
        isDefault == other.isDefault;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, card.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, isDefault.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentDetail')
          ..add('card', card)
          ..add('id', id)
          ..add('isDefault', isDefault))
        .toString();
  }
}

class AgentDetailBuilder implements Builder<AgentDetail, AgentDetailBuilder> {
  _$AgentDetail? _$v;

  JsonObject? _card;
  JsonObject? get card => _$this._card;
  set card(JsonObject? card) => _$this._card = card;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  bool? _isDefault;
  bool? get isDefault => _$this._isDefault;
  set isDefault(bool? isDefault) => _$this._isDefault = isDefault;

  AgentDetailBuilder() {
    AgentDetail._defaults(this);
  }

  AgentDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _card = $v.card;
      _id = $v.id;
      _isDefault = $v.isDefault;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentDetail other) {
    _$v = other as _$AgentDetail;
  }

  @override
  void update(void Function(AgentDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentDetail build() => _build();

  _$AgentDetail _build() {
    final _$result = _$v ??
        _$AgentDetail._(
          card: card,
          id: BuiltValueNullFieldError.checkNotNull(id, r'AgentDetail', 'id'),
          isDefault: BuiltValueNullFieldError.checkNotNull(
              isDefault, r'AgentDetail', 'isDefault'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
