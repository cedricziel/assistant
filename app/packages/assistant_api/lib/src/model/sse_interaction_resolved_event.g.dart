// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_interaction_resolved_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseInteractionResolvedEvent extends SseInteractionResolvedEvent {
  @override
  final String interactionId;
  @override
  final String kind;
  @override
  final InteractionOutcomeView outcome;
  @override
  final DateTime resolvedAt;
  @override
  final JsonObject? response;
  @override
  final String? ruleId;

  factory _$SseInteractionResolvedEvent(
          [void Function(SseInteractionResolvedEventBuilder)? updates]) =>
      (SseInteractionResolvedEventBuilder()..update(updates))._build();

  _$SseInteractionResolvedEvent._(
      {required this.interactionId,
      required this.kind,
      required this.outcome,
      required this.resolvedAt,
      this.response,
      this.ruleId})
      : super._();
  @override
  SseInteractionResolvedEvent rebuild(
          void Function(SseInteractionResolvedEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseInteractionResolvedEventBuilder toBuilder() =>
      SseInteractionResolvedEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseInteractionResolvedEvent &&
        interactionId == other.interactionId &&
        kind == other.kind &&
        outcome == other.outcome &&
        resolvedAt == other.resolvedAt &&
        response == other.response &&
        ruleId == other.ruleId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, interactionId.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jc(_$hash, outcome.hashCode);
    _$hash = $jc(_$hash, resolvedAt.hashCode);
    _$hash = $jc(_$hash, response.hashCode);
    _$hash = $jc(_$hash, ruleId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseInteractionResolvedEvent')
          ..add('interactionId', interactionId)
          ..add('kind', kind)
          ..add('outcome', outcome)
          ..add('resolvedAt', resolvedAt)
          ..add('response', response)
          ..add('ruleId', ruleId))
        .toString();
  }
}

class SseInteractionResolvedEventBuilder
    implements
        Builder<SseInteractionResolvedEvent,
            SseInteractionResolvedEventBuilder> {
  _$SseInteractionResolvedEvent? _$v;

  String? _interactionId;
  String? get interactionId => _$this._interactionId;
  set interactionId(String? interactionId) =>
      _$this._interactionId = interactionId;

  String? _kind;
  String? get kind => _$this._kind;
  set kind(String? kind) => _$this._kind = kind;

  InteractionOutcomeView? _outcome;
  InteractionOutcomeView? get outcome => _$this._outcome;
  set outcome(InteractionOutcomeView? outcome) => _$this._outcome = outcome;

  DateTime? _resolvedAt;
  DateTime? get resolvedAt => _$this._resolvedAt;
  set resolvedAt(DateTime? resolvedAt) => _$this._resolvedAt = resolvedAt;

  JsonObject? _response;
  JsonObject? get response => _$this._response;
  set response(JsonObject? response) => _$this._response = response;

  String? _ruleId;
  String? get ruleId => _$this._ruleId;
  set ruleId(String? ruleId) => _$this._ruleId = ruleId;

  SseInteractionResolvedEventBuilder() {
    SseInteractionResolvedEvent._defaults(this);
  }

  SseInteractionResolvedEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _interactionId = $v.interactionId;
      _kind = $v.kind;
      _outcome = $v.outcome;
      _resolvedAt = $v.resolvedAt;
      _response = $v.response;
      _ruleId = $v.ruleId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseInteractionResolvedEvent other) {
    _$v = other as _$SseInteractionResolvedEvent;
  }

  @override
  void update(void Function(SseInteractionResolvedEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseInteractionResolvedEvent build() => _build();

  _$SseInteractionResolvedEvent _build() {
    final _$result = _$v ??
        _$SseInteractionResolvedEvent._(
          interactionId: BuiltValueNullFieldError.checkNotNull(
              interactionId, r'SseInteractionResolvedEvent', 'interactionId'),
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'SseInteractionResolvedEvent', 'kind'),
          outcome: BuiltValueNullFieldError.checkNotNull(
              outcome, r'SseInteractionResolvedEvent', 'outcome'),
          resolvedAt: BuiltValueNullFieldError.checkNotNull(
              resolvedAt, r'SseInteractionResolvedEvent', 'resolvedAt'),
          response: response,
          ruleId: ruleId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
