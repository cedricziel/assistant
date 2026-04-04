// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'span_entry_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SpanEntryResponse extends SpanEntryResponse {
  @override
  final JsonObject? attributes;
  @override
  final int durationMs;
  @override
  final String name;
  @override
  final String spanId;
  @override
  final DateTime startTime;

  factory _$SpanEntryResponse(
          [void Function(SpanEntryResponseBuilder)? updates]) =>
      (SpanEntryResponseBuilder()..update(updates))._build();

  _$SpanEntryResponse._(
      {this.attributes,
      required this.durationMs,
      required this.name,
      required this.spanId,
      required this.startTime})
      : super._();
  @override
  SpanEntryResponse rebuild(void Function(SpanEntryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SpanEntryResponseBuilder toBuilder() =>
      SpanEntryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SpanEntryResponse &&
        attributes == other.attributes &&
        durationMs == other.durationMs &&
        name == other.name &&
        spanId == other.spanId &&
        startTime == other.startTime;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, attributes.hashCode);
    _$hash = $jc(_$hash, durationMs.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, spanId.hashCode);
    _$hash = $jc(_$hash, startTime.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SpanEntryResponse')
          ..add('attributes', attributes)
          ..add('durationMs', durationMs)
          ..add('name', name)
          ..add('spanId', spanId)
          ..add('startTime', startTime))
        .toString();
  }
}

class SpanEntryResponseBuilder
    implements Builder<SpanEntryResponse, SpanEntryResponseBuilder> {
  _$SpanEntryResponse? _$v;

  JsonObject? _attributes;
  JsonObject? get attributes => _$this._attributes;
  set attributes(JsonObject? attributes) => _$this._attributes = attributes;

  int? _durationMs;
  int? get durationMs => _$this._durationMs;
  set durationMs(int? durationMs) => _$this._durationMs = durationMs;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _spanId;
  String? get spanId => _$this._spanId;
  set spanId(String? spanId) => _$this._spanId = spanId;

  DateTime? _startTime;
  DateTime? get startTime => _$this._startTime;
  set startTime(DateTime? startTime) => _$this._startTime = startTime;

  SpanEntryResponseBuilder() {
    SpanEntryResponse._defaults(this);
  }

  SpanEntryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _attributes = $v.attributes;
      _durationMs = $v.durationMs;
      _name = $v.name;
      _spanId = $v.spanId;
      _startTime = $v.startTime;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SpanEntryResponse other) {
    _$v = other as _$SpanEntryResponse;
  }

  @override
  void update(void Function(SpanEntryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SpanEntryResponse build() => _build();

  _$SpanEntryResponse _build() {
    final _$result = _$v ??
        _$SpanEntryResponse._(
          attributes: attributes,
          durationMs: BuiltValueNullFieldError.checkNotNull(
              durationMs, r'SpanEntryResponse', 'durationMs'),
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SpanEntryResponse', 'name'),
          spanId: BuiltValueNullFieldError.checkNotNull(
              spanId, r'SpanEntryResponse', 'spanId'),
          startTime: BuiltValueNullFieldError.checkNotNull(
              startTime, r'SpanEntryResponse', 'startTime'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
