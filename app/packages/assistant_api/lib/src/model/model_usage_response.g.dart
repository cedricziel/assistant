// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'model_usage_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ModelUsageResponse extends ModelUsageResponse {
  @override
  final double avgDurationS;
  @override
  final int inputTokens;
  @override
  final String model;
  @override
  final int outputTokens;
  @override
  final int requestCount;

  factory _$ModelUsageResponse(
          [void Function(ModelUsageResponseBuilder)? updates]) =>
      (ModelUsageResponseBuilder()..update(updates))._build();

  _$ModelUsageResponse._(
      {required this.avgDurationS,
      required this.inputTokens,
      required this.model,
      required this.outputTokens,
      required this.requestCount})
      : super._();
  @override
  ModelUsageResponse rebuild(
          void Function(ModelUsageResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ModelUsageResponseBuilder toBuilder() =>
      ModelUsageResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ModelUsageResponse &&
        avgDurationS == other.avgDurationS &&
        inputTokens == other.inputTokens &&
        model == other.model &&
        outputTokens == other.outputTokens &&
        requestCount == other.requestCount;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, avgDurationS.hashCode);
    _$hash = $jc(_$hash, inputTokens.hashCode);
    _$hash = $jc(_$hash, model.hashCode);
    _$hash = $jc(_$hash, outputTokens.hashCode);
    _$hash = $jc(_$hash, requestCount.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ModelUsageResponse')
          ..add('avgDurationS', avgDurationS)
          ..add('inputTokens', inputTokens)
          ..add('model', model)
          ..add('outputTokens', outputTokens)
          ..add('requestCount', requestCount))
        .toString();
  }
}

class ModelUsageResponseBuilder
    implements Builder<ModelUsageResponse, ModelUsageResponseBuilder> {
  _$ModelUsageResponse? _$v;

  double? _avgDurationS;
  double? get avgDurationS => _$this._avgDurationS;
  set avgDurationS(double? avgDurationS) => _$this._avgDurationS = avgDurationS;

  int? _inputTokens;
  int? get inputTokens => _$this._inputTokens;
  set inputTokens(int? inputTokens) => _$this._inputTokens = inputTokens;

  String? _model;
  String? get model => _$this._model;
  set model(String? model) => _$this._model = model;

  int? _outputTokens;
  int? get outputTokens => _$this._outputTokens;
  set outputTokens(int? outputTokens) => _$this._outputTokens = outputTokens;

  int? _requestCount;
  int? get requestCount => _$this._requestCount;
  set requestCount(int? requestCount) => _$this._requestCount = requestCount;

  ModelUsageResponseBuilder() {
    ModelUsageResponse._defaults(this);
  }

  ModelUsageResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _avgDurationS = $v.avgDurationS;
      _inputTokens = $v.inputTokens;
      _model = $v.model;
      _outputTokens = $v.outputTokens;
      _requestCount = $v.requestCount;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ModelUsageResponse other) {
    _$v = other as _$ModelUsageResponse;
  }

  @override
  void update(void Function(ModelUsageResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ModelUsageResponse build() => _build();

  _$ModelUsageResponse _build() {
    final _$result = _$v ??
        _$ModelUsageResponse._(
          avgDurationS: BuiltValueNullFieldError.checkNotNull(
              avgDurationS, r'ModelUsageResponse', 'avgDurationS'),
          inputTokens: BuiltValueNullFieldError.checkNotNull(
              inputTokens, r'ModelUsageResponse', 'inputTokens'),
          model: BuiltValueNullFieldError.checkNotNull(
              model, r'ModelUsageResponse', 'model'),
          outputTokens: BuiltValueNullFieldError.checkNotNull(
              outputTokens, r'ModelUsageResponse', 'outputTokens'),
          requestCount: BuiltValueNullFieldError.checkNotNull(
              requestCount, r'ModelUsageResponse', 'requestCount'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
