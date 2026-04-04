// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trace_detail_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TraceDetailResponse extends TraceDetailResponse {
  @override
  final int durationMs;
  @override
  final String personaId;
  @override
  final BuiltList<SpanEntryResponse> spans;
  @override
  final DateTime startTime;
  @override
  final String traceId;

  factory _$TraceDetailResponse(
          [void Function(TraceDetailResponseBuilder)? updates]) =>
      (TraceDetailResponseBuilder()..update(updates))._build();

  _$TraceDetailResponse._(
      {required this.durationMs,
      required this.personaId,
      required this.spans,
      required this.startTime,
      required this.traceId})
      : super._();
  @override
  TraceDetailResponse rebuild(
          void Function(TraceDetailResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TraceDetailResponseBuilder toBuilder() =>
      TraceDetailResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TraceDetailResponse &&
        durationMs == other.durationMs &&
        personaId == other.personaId &&
        spans == other.spans &&
        startTime == other.startTime &&
        traceId == other.traceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, durationMs.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jc(_$hash, spans.hashCode);
    _$hash = $jc(_$hash, startTime.hashCode);
    _$hash = $jc(_$hash, traceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TraceDetailResponse')
          ..add('durationMs', durationMs)
          ..add('personaId', personaId)
          ..add('spans', spans)
          ..add('startTime', startTime)
          ..add('traceId', traceId))
        .toString();
  }
}

class TraceDetailResponseBuilder
    implements Builder<TraceDetailResponse, TraceDetailResponseBuilder> {
  _$TraceDetailResponse? _$v;

  int? _durationMs;
  int? get durationMs => _$this._durationMs;
  set durationMs(int? durationMs) => _$this._durationMs = durationMs;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  ListBuilder<SpanEntryResponse>? _spans;
  ListBuilder<SpanEntryResponse> get spans =>
      _$this._spans ??= ListBuilder<SpanEntryResponse>();
  set spans(ListBuilder<SpanEntryResponse>? spans) => _$this._spans = spans;

  DateTime? _startTime;
  DateTime? get startTime => _$this._startTime;
  set startTime(DateTime? startTime) => _$this._startTime = startTime;

  String? _traceId;
  String? get traceId => _$this._traceId;
  set traceId(String? traceId) => _$this._traceId = traceId;

  TraceDetailResponseBuilder() {
    TraceDetailResponse._defaults(this);
  }

  TraceDetailResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _durationMs = $v.durationMs;
      _personaId = $v.personaId;
      _spans = $v.spans.toBuilder();
      _startTime = $v.startTime;
      _traceId = $v.traceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TraceDetailResponse other) {
    _$v = other as _$TraceDetailResponse;
  }

  @override
  void update(void Function(TraceDetailResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TraceDetailResponse build() => _build();

  _$TraceDetailResponse _build() {
    _$TraceDetailResponse _$result;
    try {
      _$result = _$v ??
          _$TraceDetailResponse._(
            durationMs: BuiltValueNullFieldError.checkNotNull(
                durationMs, r'TraceDetailResponse', 'durationMs'),
            personaId: BuiltValueNullFieldError.checkNotNull(
                personaId, r'TraceDetailResponse', 'personaId'),
            spans: spans.build(),
            startTime: BuiltValueNullFieldError.checkNotNull(
                startTime, r'TraceDetailResponse', 'startTime'),
            traceId: BuiltValueNullFieldError.checkNotNull(
                traceId, r'TraceDetailResponse', 'traceId'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'spans';
        spans.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TraceDetailResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
