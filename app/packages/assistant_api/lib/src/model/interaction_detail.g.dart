// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'interaction_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$InteractionDetail extends InteractionDetail {
  @override
  final String conversationId;
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
  final JsonObject request;
  @override
  final DateTime? resolvedAt;
  @override
  final JsonObject? response;
  @override
  final String? ruleId;
  @override
  final String runId;
  @override
  final InteractionStatusView status;
  @override
  final String? toolCallId;

  factory _$InteractionDetail(
          [void Function(InteractionDetailBuilder)? updates]) =>
      (InteractionDetailBuilder()..update(updates))._build();

  _$InteractionDetail._(
      {required this.conversationId,
      required this.createdAt,
      required this.expiresAt,
      required this.interactionId,
      required this.kind,
      required this.rendersCard,
      required this.request,
      this.resolvedAt,
      this.response,
      this.ruleId,
      required this.runId,
      required this.status,
      this.toolCallId})
      : super._();
  @override
  InteractionDetail rebuild(void Function(InteractionDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  InteractionDetailBuilder toBuilder() =>
      InteractionDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is InteractionDetail &&
        conversationId == other.conversationId &&
        createdAt == other.createdAt &&
        expiresAt == other.expiresAt &&
        interactionId == other.interactionId &&
        kind == other.kind &&
        rendersCard == other.rendersCard &&
        request == other.request &&
        resolvedAt == other.resolvedAt &&
        response == other.response &&
        ruleId == other.ruleId &&
        runId == other.runId &&
        status == other.status &&
        toolCallId == other.toolCallId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, conversationId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, interactionId.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jc(_$hash, rendersCard.hashCode);
    _$hash = $jc(_$hash, request.hashCode);
    _$hash = $jc(_$hash, resolvedAt.hashCode);
    _$hash = $jc(_$hash, response.hashCode);
    _$hash = $jc(_$hash, ruleId.hashCode);
    _$hash = $jc(_$hash, runId.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, toolCallId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'InteractionDetail')
          ..add('conversationId', conversationId)
          ..add('createdAt', createdAt)
          ..add('expiresAt', expiresAt)
          ..add('interactionId', interactionId)
          ..add('kind', kind)
          ..add('rendersCard', rendersCard)
          ..add('request', request)
          ..add('resolvedAt', resolvedAt)
          ..add('response', response)
          ..add('ruleId', ruleId)
          ..add('runId', runId)
          ..add('status', status)
          ..add('toolCallId', toolCallId))
        .toString();
  }
}

class InteractionDetailBuilder
    implements Builder<InteractionDetail, InteractionDetailBuilder> {
  _$InteractionDetail? _$v;

  String? _conversationId;
  String? get conversationId => _$this._conversationId;
  set conversationId(String? conversationId) =>
      _$this._conversationId = conversationId;

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

  JsonObject? _request;
  JsonObject? get request => _$this._request;
  set request(JsonObject? request) => _$this._request = request;

  DateTime? _resolvedAt;
  DateTime? get resolvedAt => _$this._resolvedAt;
  set resolvedAt(DateTime? resolvedAt) => _$this._resolvedAt = resolvedAt;

  JsonObject? _response;
  JsonObject? get response => _$this._response;
  set response(JsonObject? response) => _$this._response = response;

  String? _ruleId;
  String? get ruleId => _$this._ruleId;
  set ruleId(String? ruleId) => _$this._ruleId = ruleId;

  String? _runId;
  String? get runId => _$this._runId;
  set runId(String? runId) => _$this._runId = runId;

  InteractionStatusView? _status;
  InteractionStatusView? get status => _$this._status;
  set status(InteractionStatusView? status) => _$this._status = status;

  String? _toolCallId;
  String? get toolCallId => _$this._toolCallId;
  set toolCallId(String? toolCallId) => _$this._toolCallId = toolCallId;

  InteractionDetailBuilder() {
    InteractionDetail._defaults(this);
  }

  InteractionDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _conversationId = $v.conversationId;
      _createdAt = $v.createdAt;
      _expiresAt = $v.expiresAt;
      _interactionId = $v.interactionId;
      _kind = $v.kind;
      _rendersCard = $v.rendersCard;
      _request = $v.request;
      _resolvedAt = $v.resolvedAt;
      _response = $v.response;
      _ruleId = $v.ruleId;
      _runId = $v.runId;
      _status = $v.status;
      _toolCallId = $v.toolCallId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(InteractionDetail other) {
    _$v = other as _$InteractionDetail;
  }

  @override
  void update(void Function(InteractionDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  InteractionDetail build() => _build();

  _$InteractionDetail _build() {
    final _$result = _$v ??
        _$InteractionDetail._(
          conversationId: BuiltValueNullFieldError.checkNotNull(
              conversationId, r'InteractionDetail', 'conversationId'),
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'InteractionDetail', 'createdAt'),
          expiresAt: BuiltValueNullFieldError.checkNotNull(
              expiresAt, r'InteractionDetail', 'expiresAt'),
          interactionId: BuiltValueNullFieldError.checkNotNull(
              interactionId, r'InteractionDetail', 'interactionId'),
          kind: BuiltValueNullFieldError.checkNotNull(
              kind, r'InteractionDetail', 'kind'),
          rendersCard: BuiltValueNullFieldError.checkNotNull(
              rendersCard, r'InteractionDetail', 'rendersCard'),
          request: BuiltValueNullFieldError.checkNotNull(
              request, r'InteractionDetail', 'request'),
          resolvedAt: resolvedAt,
          response: response,
          ruleId: ruleId,
          runId: BuiltValueNullFieldError.checkNotNull(
              runId, r'InteractionDetail', 'runId'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'InteractionDetail', 'status'),
          toolCallId: toolCallId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
