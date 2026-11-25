// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'test_integration_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TestIntegrationResponse extends TestIntegrationResponse {
  @override
  final String? error;
  @override
  final DateTime? lastConnectedAt;
  @override
  final bool ok;
  @override
  final IntegrationStatus status;
  @override
  final int? toolCount;
  @override
  final BuiltList<String> toolNames;

  factory _$TestIntegrationResponse(
          [void Function(TestIntegrationResponseBuilder)? updates]) =>
      (TestIntegrationResponseBuilder()..update(updates))._build();

  _$TestIntegrationResponse._(
      {this.error,
      this.lastConnectedAt,
      required this.ok,
      required this.status,
      this.toolCount,
      required this.toolNames})
      : super._();
  @override
  TestIntegrationResponse rebuild(
          void Function(TestIntegrationResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  TestIntegrationResponseBuilder toBuilder() =>
      TestIntegrationResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TestIntegrationResponse &&
        error == other.error &&
        lastConnectedAt == other.lastConnectedAt &&
        ok == other.ok &&
        status == other.status &&
        toolCount == other.toolCount &&
        toolNames == other.toolNames;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, error.hashCode);
    _$hash = $jc(_$hash, lastConnectedAt.hashCode);
    _$hash = $jc(_$hash, ok.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, toolCount.hashCode);
    _$hash = $jc(_$hash, toolNames.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TestIntegrationResponse')
          ..add('error', error)
          ..add('lastConnectedAt', lastConnectedAt)
          ..add('ok', ok)
          ..add('status', status)
          ..add('toolCount', toolCount)
          ..add('toolNames', toolNames))
        .toString();
  }
}

class TestIntegrationResponseBuilder
    implements
        Builder<TestIntegrationResponse, TestIntegrationResponseBuilder> {
  _$TestIntegrationResponse? _$v;

  String? _error;
  String? get error => _$this._error;
  set error(String? error) => _$this._error = error;

  DateTime? _lastConnectedAt;
  DateTime? get lastConnectedAt => _$this._lastConnectedAt;
  set lastConnectedAt(DateTime? lastConnectedAt) =>
      _$this._lastConnectedAt = lastConnectedAt;

  bool? _ok;
  bool? get ok => _$this._ok;
  set ok(bool? ok) => _$this._ok = ok;

  IntegrationStatus? _status;
  IntegrationStatus? get status => _$this._status;
  set status(IntegrationStatus? status) => _$this._status = status;

  int? _toolCount;
  int? get toolCount => _$this._toolCount;
  set toolCount(int? toolCount) => _$this._toolCount = toolCount;

  ListBuilder<String>? _toolNames;
  ListBuilder<String> get toolNames =>
      _$this._toolNames ??= ListBuilder<String>();
  set toolNames(ListBuilder<String>? toolNames) =>
      _$this._toolNames = toolNames;

  TestIntegrationResponseBuilder() {
    TestIntegrationResponse._defaults(this);
  }

  TestIntegrationResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _error = $v.error;
      _lastConnectedAt = $v.lastConnectedAt;
      _ok = $v.ok;
      _status = $v.status;
      _toolCount = $v.toolCount;
      _toolNames = $v.toolNames.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TestIntegrationResponse other) {
    _$v = other as _$TestIntegrationResponse;
  }

  @override
  void update(void Function(TestIntegrationResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TestIntegrationResponse build() => _build();

  _$TestIntegrationResponse _build() {
    _$TestIntegrationResponse _$result;
    try {
      _$result = _$v ??
          _$TestIntegrationResponse._(
            error: error,
            lastConnectedAt: lastConnectedAt,
            ok: BuiltValueNullFieldError.checkNotNull(
                ok, r'TestIntegrationResponse', 'ok'),
            status: BuiltValueNullFieldError.checkNotNull(
                status, r'TestIntegrationResponse', 'status'),
            toolCount: toolCount,
            toolNames: toolNames.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'toolNames';
        toolNames.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'TestIntegrationResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
