// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trace_summary_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TraceSummaryResponse extends TraceSummaryResponse {
  @override
  final String? conversationId;
  @override
  final int durationMs;
  @override
  final String personaId;
  @override
  final String? skillName;
  @override
  final DateTime startTime;
  @override
  final String status;
  @override
  final String traceId;

  factory _$TraceSummaryResponse(
          [void Function(TraceSummaryResponseBuilder)? updates]) =>
      (TraceSummaryResponseBuilder()..update(updates))._build();

  _$TraceSummaryResponse._(
      {this.conversationId,
      required this.durationMs,
      required this.personaId,
      this.skillName,
      required this.startTime,
      required this.status,
      required this.traceId})
      : super._();
  @override
  TraceSummaryResponse rebuild(
          void Function(TraceSummaryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TraceSummaryResponseBuilder toBuilder() =>
      TraceSummaryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TraceSummaryResponse &&
        conversationId == other.conversationId &&
        durationMs == other.durationMs &&
        personaId == other.personaId &&
        skillName == other.skillName &&
        startTime == other.startTime &&
        status == other.status &&
        traceId == other.traceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, conversationId.hashCode);
    _$hash = $jc(_$hash, durationMs.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jc(_$hash, skillName.hashCode);
    _$hash = $jc(_$hash, startTime.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, traceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TraceSummaryResponse')
          ..add('conversationId', conversationId)
          ..add('durationMs', durationMs)
          ..add('personaId', personaId)
          ..add('skillName', skillName)
          ..add('startTime', startTime)
          ..add('status', status)
          ..add('traceId', traceId))
        .toString();
  }
}

class TraceSummaryResponseBuilder
    implements Builder<TraceSummaryResponse, TraceSummaryResponseBuilder> {
  _$TraceSummaryResponse? _$v;

  String? _conversationId;
  String? get conversationId => _$this._conversationId;
  set conversationId(String? conversationId) =>
      _$this._conversationId = conversationId;

  int? _durationMs;
  int? get durationMs => _$this._durationMs;
  set durationMs(int? durationMs) => _$this._durationMs = durationMs;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  String? _skillName;
  String? get skillName => _$this._skillName;
  set skillName(String? skillName) => _$this._skillName = skillName;

  DateTime? _startTime;
  DateTime? get startTime => _$this._startTime;
  set startTime(DateTime? startTime) => _$this._startTime = startTime;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _traceId;
  String? get traceId => _$this._traceId;
  set traceId(String? traceId) => _$this._traceId = traceId;

  TraceSummaryResponseBuilder() {
    TraceSummaryResponse._defaults(this);
  }

  TraceSummaryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _conversationId = $v.conversationId;
      _durationMs = $v.durationMs;
      _personaId = $v.personaId;
      _skillName = $v.skillName;
      _startTime = $v.startTime;
      _status = $v.status;
      _traceId = $v.traceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TraceSummaryResponse other) {
    _$v = other as _$TraceSummaryResponse;
  }

  @override
  void update(void Function(TraceSummaryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TraceSummaryResponse build() => _build();

  _$TraceSummaryResponse _build() {
    final _$result = _$v ??
        _$TraceSummaryResponse._(
          conversationId: conversationId,
          durationMs: BuiltValueNullFieldError.checkNotNull(
              durationMs, r'TraceSummaryResponse', 'durationMs'),
          personaId: BuiltValueNullFieldError.checkNotNull(
              personaId, r'TraceSummaryResponse', 'personaId'),
          skillName: skillName,
          startTime: BuiltValueNullFieldError.checkNotNull(
              startTime, r'TraceSummaryResponse', 'startTime'),
          status: BuiltValueNullFieldError.checkNotNull(
              status, r'TraceSummaryResponse', 'status'),
          traceId: BuiltValueNullFieldError.checkNotNull(
              traceId, r'TraceSummaryResponse', 'traceId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
