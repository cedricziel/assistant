// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_interaction_requested_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseInteractionRequestedEvent extends SseInteractionRequestedEvent {
  @override
  final DateTime createdAt;
  @override
  final DateTime expiresAt;
  @override
  final String interactionId;
  @override
  final String kind;
  @override
  final bool rendersCard;
  @override
  final String? toolCallId;

  factory _$SseInteractionRequestedEvent(
          [void Function(SseInteractionRequestedEventBuilder)? updates]) =>
      (SseInteractionRequestedEventBuilder()..update(updates))._build();

  _$SseInteractionRequestedEvent._(
      {required this.createdAt,
      required this.expiresAt,
      required this.interactionId,
      required this.kind,
      required this.rendersCard,
      this.toolCallId})
      : super._();
  @override
  SseInteractionRequestedEvent rebuild(
          void Function(SseInteractionRequestedEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseInteractionRequestedEventBuilder toBuilder() =>
      SseInteractionRequestedEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseInteractionRequestedEvent &&
        createdAt == other.createdAt &&
        expiresAt == other.expiresAt &&
        interactionId == other.interactionId &&
        kind == other.kind &&
        rendersCard == other.rendersCard &&
        toolCallId == other.toolCallId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, interactionId.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jc(_$hash, rendersCard.hashCode);
    _$hash = $jc(_$hash, toolCallId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseInteractionRequestedEvent')
          ..add('createdAt', createdAt)
          ..add('expiresAt', expiresAt)
          ..add('interactionId', interactionId)
          ..add('kind', kind)
          ..add('rendersCard', rendersCard)
          ..add('toolCallId', toolCallId))
        .toString();
  }
}

class SseInteractionRequestedEventBuilder
    implements
        Builder<SseInteractionRequestedEvent,
            SseInteractionRequestedEventBuilder> {
  _$SseInteractionRequestedEvent? _$v;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  DateTime? _expiresAt;
  DateTime? get expiresAt => _$this._expiresAt;
  set expiresAt(DateTime? expiresAt) => _$this._expiresAt = expiresAt;

  String? _interactionId;
  String? get interactionId => _$this._interactionId;
  set interactionId(String? interactionId) =>
      _$this._interactionId = interactionId;

  String? _kind;
  String? get kind => _$this._kind;
  set kind(String? kind) => _$this._kind = kind;

  bool? _rendersCard;
  bool? get rendersCard => _$this._rendersCard;
  set rendersCard(bool? rendersCard) => _$this._rendersCard = rendersCard;

  String? _toolCallId;
  String? get toolCallId => _$this._toolCallId;
  set toolCallId(String? toolCallId) => _$this._toolCallId = toolCallId;

  SseInteractionRequestedEventBuilder() {
    SseInteractionRequestedEvent._defaults(this);
  }

  SseInteractionRequestedEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _expiresAt = $v.expiresAt;
      _interactionId = $v.interactionId;
      _kind = $v.kind;
      _rendersCard = $v.rendersCard;
      _toolCallId = $v.toolCallId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseInteractionRequestedEvent other) {
    _$v = other as _$SseInteractionRequestedEvent;
  }

  @override
  void update(void Function(SseInteractionRequestedEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseInteractionRequestedEvent build() => _build();

  _$SseInteractionRequestedEvent _build() {
    final _$result = _$v ??
        _$SseInteractionRequestedEvent._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'SseInteractionRequestedEvent', 'createdAt'),
          expiresAt: BuiltValueNullFieldError.checkNotNull(
              expiresAt, r'SseInteractionRequestedEvent', 'expiresAt'),
          interactionId: BuiltValueNullFieldError.checkNotNull(
              interactionId, r'SseInteractionRequestedEvent', 'interactionId'),
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'SseInteractionRequestedEvent', 'kind'),
          rendersCard: BuiltValueNullFieldError.checkNotNull(
              rendersCard, r'SseInteractionRequestedEvent', 'rendersCard'),
          toolCallId: toolCallId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
