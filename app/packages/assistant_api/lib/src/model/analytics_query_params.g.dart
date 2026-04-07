// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'analytics_query_params.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AnalyticsQueryParams extends AnalyticsQueryParams {
  @override
  final int? window;

  factory _$AnalyticsQueryParams(
          [void Function(AnalyticsQueryParamsBuilder)? updates]) =>
      (AnalyticsQueryParamsBuilder()..update(updates))._build();

  _$AnalyticsQueryParams._({this.window}) : super._();
  @override
  AnalyticsQueryParams rebuild(
          void Function(AnalyticsQueryParamsBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AnalyticsQueryParamsBuilder toBuilder() =>
      AnalyticsQueryParamsBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AnalyticsQueryParams && window == other.window;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, window.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AnalyticsQueryParams')
          ..add('window', window))
        .toString();
  }
}

class AnalyticsQueryParamsBuilder
    implements Builder<AnalyticsQueryParams, AnalyticsQueryParamsBuilder> {
  _$AnalyticsQueryParams? _$v;

  int? _window;
  int? get window => _$this._window;
  set window(int? window) => _$this._window = window;

  AnalyticsQueryParamsBuilder() {
    AnalyticsQueryParams._defaults(this);
  }

  AnalyticsQueryParamsBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _window = $v.window;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AnalyticsQueryParams other) {
    _$v = other as _$AnalyticsQueryParams;
  }

  @override
  void update(void Function(AnalyticsQueryParamsBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AnalyticsQueryParams build() => _build();

  _$AnalyticsQueryParams _build() {
    final _$result = _$v ??
        _$AnalyticsQueryParams._(
          window: window,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
