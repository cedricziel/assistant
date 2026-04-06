// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tool_usage_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ToolUsageResponse extends ToolUsageResponse {
  @override
  final double avgDurationS;
  @override
  final int invocations;
  @override
  final String toolName;

  factory _$ToolUsageResponse(
          [void Function(ToolUsageResponseBuilder)? updates]) =>
      (ToolUsageResponseBuilder()..update(updates))._build();

  _$ToolUsageResponse._(
      {required this.avgDurationS,
      required this.invocations,
      required this.toolName})
      : super._();
  @override
  ToolUsageResponse rebuild(void Function(ToolUsageResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ToolUsageResponseBuilder toBuilder() =>
      ToolUsageResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ToolUsageResponse &&
        avgDurationS == other.avgDurationS &&
        invocations == other.invocations &&
        toolName == other.toolName;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, avgDurationS.hashCode);
    _$hash = $jc(_$hash, invocations.hashCode);
    _$hash = $jc(_$hash, toolName.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ToolUsageResponse')
          ..add('avgDurationS', avgDurationS)
          ..add('invocations', invocations)
          ..add('toolName', toolName))
        .toString();
  }
}

class ToolUsageResponseBuilder
    implements Builder<ToolUsageResponse, ToolUsageResponseBuilder> {
  _$ToolUsageResponse? _$v;

  double? _avgDurationS;
  double? get avgDurationS => _$this._avgDurationS;
  set avgDurationS(double? avgDurationS) => _$this._avgDurationS = avgDurationS;

  int? _invocations;
  int? get invocations => _$this._invocations;
  set invocations(int? invocations) => _$this._invocations = invocations;

  String? _toolName;
  String? get toolName => _$this._toolName;
  set toolName(String? toolName) => _$this._toolName = toolName;

  ToolUsageResponseBuilder() {
    ToolUsageResponse._defaults(this);
  }

  ToolUsageResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _avgDurationS = $v.avgDurationS;
      _invocations = $v.invocations;
      _toolName = $v.toolName;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ToolUsageResponse other) {
    _$v = other as _$ToolUsageResponse;
  }

  @override
  void update(void Function(ToolUsageResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ToolUsageResponse build() => _build();

  _$ToolUsageResponse _build() {
    final _$result = _$v ??
        _$ToolUsageResponse._(
          avgDurationS: BuiltValueNullFieldError.checkNotNull(
              avgDurationS, r'ToolUsageResponse', 'avgDurationS'),
          invocations: BuiltValueNullFieldError.checkNotNull(
              invocations, r'ToolUsageResponse', 'invocations'),
          toolName: BuiltValueNullFieldError.checkNotNull(
              toolName, r'ToolUsageResponse', 'toolName'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
