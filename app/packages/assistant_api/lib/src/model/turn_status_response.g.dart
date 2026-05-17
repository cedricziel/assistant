// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'turn_status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TurnStatusResponse extends TurnStatusResponse {
  @override
  final String conversationId;
  @override
  final String? lastEventAt;
  @override
  final String? lastEventKind;
  @override
  final TurnState state;
  @override
  final String turnId;

  factory _$TurnStatusResponse(
          [void Function(TurnStatusResponseBuilder)? updates]) =>
      (TurnStatusResponseBuilder()..update(updates))._build();

  _$TurnStatusResponse._(
      {required this.conversationId,
      this.lastEventAt,
      this.lastEventKind,
      required this.state,
      required this.turnId})
      : super._();
  @override
  TurnStatusResponse rebuild(
          void Function(TurnStatusResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TurnStatusResponseBuilder toBuilder() =>
      TurnStatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TurnStatusResponse &&
        conversationId == other.conversationId &&
        lastEventAt == other.lastEventAt &&
        lastEventKind == other.lastEventKind &&
        state == other.state &&
        turnId == other.turnId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, conversationId.hashCode);
    _$hash = $jc(_$hash, lastEventAt.hashCode);
    _$hash = $jc(_$hash, lastEventKind.hashCode);
    _$hash = $jc(_$hash, state.hashCode);
    _$hash = $jc(_$hash, turnId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TurnStatusResponse')
          ..add('conversationId', conversationId)
          ..add('lastEventAt', lastEventAt)
          ..add('lastEventKind', lastEventKind)
          ..add('state', state)
          ..add('turnId', turnId))
        .toString();
  }
}

class TurnStatusResponseBuilder
    implements Builder<TurnStatusResponse, TurnStatusResponseBuilder> {
  _$TurnStatusResponse? _$v;

  String? _conversationId;
  String? get conversationId => _$this._conversationId;
  set conversationId(String? conversationId) =>
      _$this._conversationId = conversationId;

  String? _lastEventAt;
  String? get lastEventAt => _$this._lastEventAt;
  set lastEventAt(String? lastEventAt) => _$this._lastEventAt = lastEventAt;

  String? _lastEventKind;
  String? get lastEventKind => _$this._lastEventKind;
  set lastEventKind(String? lastEventKind) =>
      _$this._lastEventKind = lastEventKind;

  TurnState? _state;
  TurnState? get state => _$this._state;
  set state(TurnState? state) => _$this._state = state;

  String? _turnId;
  String? get turnId => _$this._turnId;
  set turnId(String? turnId) => _$this._turnId = turnId;

  TurnStatusResponseBuilder() {
    TurnStatusResponse._defaults(this);
  }

  TurnStatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _conversationId = $v.conversationId;
      _lastEventAt = $v.lastEventAt;
      _lastEventKind = $v.lastEventKind;
      _state = $v.state;
      _turnId = $v.turnId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TurnStatusResponse other) {
    _$v = other as _$TurnStatusResponse;
  }

  @override
  void update(void Function(TurnStatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TurnStatusResponse build() => _build();

  _$TurnStatusResponse _build() {
    final _$result = _$v ??
        _$TurnStatusResponse._(
          conversationId: BuiltValueNullFieldError.checkNotNull(
              conversationId, r'TurnStatusResponse', 'conversationId'),
          lastEventAt: lastEventAt,
          lastEventKind: lastEventKind,
          state: BuiltValueNullFieldError.checkNotNull(
              state, r'TurnStatusResponse', 'state'),
          turnId: BuiltValueNullFieldError.checkNotNull(
              turnId, r'TurnStatusResponse', 'turnId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
