// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'time_series_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TimeSeriesResponse extends TimeSeriesResponse {
  @override
  final String bucket;
  @override
  final double value;

  factory _$TimeSeriesResponse(
          [void Function(TimeSeriesResponseBuilder)? updates]) =>
      (TimeSeriesResponseBuilder()..update(updates))._build();

  _$TimeSeriesResponse._({required this.bucket, required this.value})
      : super._();
  @override
  TimeSeriesResponse rebuild(
          void Function(TimeSeriesResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TimeSeriesResponseBuilder toBuilder() =>
      TimeSeriesResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TimeSeriesResponse &&
        bucket == other.bucket &&
        value == other.value;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, bucket.hashCode);
    _$hash = $jc(_$hash, value.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TimeSeriesResponse')
          ..add('bucket', bucket)
          ..add('value', value))
        .toString();
  }
}

class TimeSeriesResponseBuilder
    implements Builder<TimeSeriesResponse, TimeSeriesResponseBuilder> {
  _$TimeSeriesResponse? _$v;

  String? _bucket;
  String? get bucket => _$this._bucket;
  set bucket(String? bucket) => _$this._bucket = bucket;

  double? _value;
  double? get value => _$this._value;
  set value(double? value) => _$this._value = value;

  TimeSeriesResponseBuilder() {
    TimeSeriesResponse._defaults(this);
  }

  TimeSeriesResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _bucket = $v.bucket;
      _value = $v.value;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TimeSeriesResponse other) {
    _$v = other as _$TimeSeriesResponse;
  }

  @override
  void update(void Function(TimeSeriesResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TimeSeriesResponse build() => _build();

  _$TimeSeriesResponse _build() {
    final _$result = _$v ??
        _$TimeSeriesResponse._(
          bucket: BuiltValueNullFieldError.checkNotNull(
              bucket, r'TimeSeriesResponse', 'bucket'),
          value: BuiltValueNullFieldError.checkNotNull(
              value, r'TimeSeriesResponse', 'value'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
