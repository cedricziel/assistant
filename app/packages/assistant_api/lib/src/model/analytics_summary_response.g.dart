// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'analytics_summary_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AnalyticsSummaryResponse extends AnalyticsSummaryResponse {
  @override
  final double avgDurationS;
  @override
  final int errorCount;
  @override
  final BuiltList<ModelUsageResponse> models;
  @override
  final BuiltList<TimeSeriesResponse> requestSeries;
  @override
  final BuiltList<TimeSeriesResponse> tokenSeries;
  @override
  final BuiltList<ToolUsageResponse> tools;
  @override
  final int totalRequests;
  @override
  final int totalTokensIn;
  @override
  final int totalTokensOut;
  @override
  final int totalToolInvocations;
  @override
  final BuiltList<String> uniqueModels;
  @override
  final int windowHours;

  factory _$AnalyticsSummaryResponse(
          [void Function(AnalyticsSummaryResponseBuilder)? updates]) =>
      (AnalyticsSummaryResponseBuilder()..update(updates))._build();

  _$AnalyticsSummaryResponse._(
      {required this.avgDurationS,
      required this.errorCount,
      required this.models,
      required this.requestSeries,
      required this.tokenSeries,
      required this.tools,
      required this.totalRequests,
      required this.totalTokensIn,
      required this.totalTokensOut,
      required this.totalToolInvocations,
      required this.uniqueModels,
      required this.windowHours})
      : super._();
  @override
  AnalyticsSummaryResponse rebuild(
          void Function(AnalyticsSummaryResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AnalyticsSummaryResponseBuilder toBuilder() =>
      AnalyticsSummaryResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AnalyticsSummaryResponse &&
        avgDurationS == other.avgDurationS &&
        errorCount == other.errorCount &&
        models == other.models &&
        requestSeries == other.requestSeries &&
        tokenSeries == other.tokenSeries &&
        tools == other.tools &&
        totalRequests == other.totalRequests &&
        totalTokensIn == other.totalTokensIn &&
        totalTokensOut == other.totalTokensOut &&
        totalToolInvocations == other.totalToolInvocations &&
        uniqueModels == other.uniqueModels &&
        windowHours == other.windowHours;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, avgDurationS.hashCode);
    _$hash = $jc(_$hash, errorCount.hashCode);
    _$hash = $jc(_$hash, models.hashCode);
    _$hash = $jc(_$hash, requestSeries.hashCode);
    _$hash = $jc(_$hash, tokenSeries.hashCode);
    _$hash = $jc(_$hash, tools.hashCode);
    _$hash = $jc(_$hash, totalRequests.hashCode);
    _$hash = $jc(_$hash, totalTokensIn.hashCode);
    _$hash = $jc(_$hash, totalTokensOut.hashCode);
    _$hash = $jc(_$hash, totalToolInvocations.hashCode);
    _$hash = $jc(_$hash, uniqueModels.hashCode);
    _$hash = $jc(_$hash, windowHours.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AnalyticsSummaryResponse')
          ..add('avgDurationS', avgDurationS)
          ..add('errorCount', errorCount)
          ..add('models', models)
          ..add('requestSeries', requestSeries)
          ..add('tokenSeries', tokenSeries)
          ..add('tools', tools)
          ..add('totalRequests', totalRequests)
          ..add('totalTokensIn', totalTokensIn)
          ..add('totalTokensOut', totalTokensOut)
          ..add('totalToolInvocations', totalToolInvocations)
          ..add('uniqueModels', uniqueModels)
          ..add('windowHours', windowHours))
        .toString();
  }
}

class AnalyticsSummaryResponseBuilder
    implements
        Builder<AnalyticsSummaryResponse, AnalyticsSummaryResponseBuilder> {
  _$AnalyticsSummaryResponse? _$v;

  double? _avgDurationS;
  double? get avgDurationS => _$this._avgDurationS;
  set avgDurationS(double? avgDurationS) => _$this._avgDurationS = avgDurationS;

  int? _errorCount;
  int? get errorCount => _$this._errorCount;
  set errorCount(int? errorCount) => _$this._errorCount = errorCount;

  ListBuilder<ModelUsageResponse>? _models;
  ListBuilder<ModelUsageResponse> get models =>
      _$this._models ??= ListBuilder<ModelUsageResponse>();
  set models(ListBuilder<ModelUsageResponse>? models) =>
      _$this._models = models;

  ListBuilder<TimeSeriesResponse>? _requestSeries;
  ListBuilder<TimeSeriesResponse> get requestSeries =>
      _$this._requestSeries ??= ListBuilder<TimeSeriesResponse>();
  set requestSeries(ListBuilder<TimeSeriesResponse>? requestSeries) =>
      _$this._requestSeries = requestSeries;

  ListBuilder<TimeSeriesResponse>? _tokenSeries;
  ListBuilder<TimeSeriesResponse> get tokenSeries =>
      _$this._tokenSeries ??= ListBuilder<TimeSeriesResponse>();
  set tokenSeries(ListBuilder<TimeSeriesResponse>? tokenSeries) =>
      _$this._tokenSeries = tokenSeries;

  ListBuilder<ToolUsageResponse>? _tools;
  ListBuilder<ToolUsageResponse> get tools =>
      _$this._tools ??= ListBuilder<ToolUsageResponse>();
  set tools(ListBuilder<ToolUsageResponse>? tools) => _$this._tools = tools;

  int? _totalRequests;
  int? get totalRequests => _$this._totalRequests;
  set totalRequests(int? totalRequests) =>
      _$this._totalRequests = totalRequests;

  int? _totalTokensIn;
  int? get totalTokensIn => _$this._totalTokensIn;
  set totalTokensIn(int? totalTokensIn) =>
      _$this._totalTokensIn = totalTokensIn;

  int? _totalTokensOut;
  int? get totalTokensOut => _$this._totalTokensOut;
  set totalTokensOut(int? totalTokensOut) =>
      _$this._totalTokensOut = totalTokensOut;

  int? _totalToolInvocations;
  int? get totalToolInvocations => _$this._totalToolInvocations;
  set totalToolInvocations(int? totalToolInvocations) =>
      _$this._totalToolInvocations = totalToolInvocations;

  ListBuilder<String>? _uniqueModels;
  ListBuilder<String> get uniqueModels =>
      _$this._uniqueModels ??= ListBuilder<String>();
  set uniqueModels(ListBuilder<String>? uniqueModels) =>
      _$this._uniqueModels = uniqueModels;

  int? _windowHours;
  int? get windowHours => _$this._windowHours;
  set windowHours(int? windowHours) => _$this._windowHours = windowHours;

  AnalyticsSummaryResponseBuilder() {
    AnalyticsSummaryResponse._defaults(this);
  }

  AnalyticsSummaryResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _avgDurationS = $v.avgDurationS;
      _errorCount = $v.errorCount;
      _models = $v.models.toBuilder();
      _requestSeries = $v.requestSeries.toBuilder();
      _tokenSeries = $v.tokenSeries.toBuilder();
      _tools = $v.tools.toBuilder();
      _totalRequests = $v.totalRequests;
      _totalTokensIn = $v.totalTokensIn;
      _totalTokensOut = $v.totalTokensOut;
      _totalToolInvocations = $v.totalToolInvocations;
      _uniqueModels = $v.uniqueModels.toBuilder();
      _windowHours = $v.windowHours;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AnalyticsSummaryResponse other) {
    _$v = other as _$AnalyticsSummaryResponse;
  }

  @override
  void update(void Function(AnalyticsSummaryResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AnalyticsSummaryResponse build() => _build();

  _$AnalyticsSummaryResponse _build() {
    _$AnalyticsSummaryResponse _$result;
    try {
      _$result = _$v ??
          _$AnalyticsSummaryResponse._(
            avgDurationS: BuiltValueNullFieldError.checkNotNull(
                avgDurationS, r'AnalyticsSummaryResponse', 'avgDurationS'),
            errorCount: BuiltValueNullFieldError.checkNotNull(
                errorCount, r'AnalyticsSummaryResponse', 'errorCount'),
            models: models.build(),
            requestSeries: requestSeries.build(),
            tokenSeries: tokenSeries.build(),
            tools: tools.build(),
            totalRequests: BuiltValueNullFieldError.checkNotNull(
                totalRequests, r'AnalyticsSummaryResponse', 'totalRequests'),
            totalTokensIn: BuiltValueNullFieldError.checkNotNull(
                totalTokensIn, r'AnalyticsSummaryResponse', 'totalTokensIn'),
            totalTokensOut: BuiltValueNullFieldError.checkNotNull(
                totalTokensOut, r'AnalyticsSummaryResponse', 'totalTokensOut'),
            totalToolInvocations: BuiltValueNullFieldError.checkNotNull(
                totalToolInvocations,
                r'AnalyticsSummaryResponse',
                'totalToolInvocations'),
            uniqueModels: uniqueModels.build(),
            windowHours: BuiltValueNullFieldError.checkNotNull(
                windowHours, r'AnalyticsSummaryResponse', 'windowHours'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'models';
        models.build();
        _$failedField = 'requestSeries';
        requestSeries.build();
        _$failedField = 'tokenSeries';
        tokenSeries.build();
        _$failedField = 'tools';
        tools.build();

        _$failedField = 'uniqueModels';
        uniqueModels.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AnalyticsSummaryResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
