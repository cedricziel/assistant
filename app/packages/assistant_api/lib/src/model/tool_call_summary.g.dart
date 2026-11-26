// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tool_call_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ToolCallSummary extends ToolCallSummary {
  @override
  final McpAppView? app;
  @override
  final JsonObject? arguments;
  @override
  final JsonObject? data;
  @override
  final String name;
  @override
  final String? result;
  @override
  final String status;
  @override
  final String? toolCallId;

  factory _$ToolCallSummary([void Function(ToolCallSummaryBuilder)? updates]) =>
      (ToolCallSummaryBuilder()..update(updates))._build();

  _$ToolCallSummary._(
      {this.app,
      this.arguments,
      this.data,
      required this.name,
      this.result,
      required this.status,
      this.toolCallId})
      : super._();
  @override
  ToolCallSummary rebuild(void Function(ToolCallSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ToolCallSummaryBuilder toBuilder() => ToolCallSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ToolCallSummary &&
        app == other.app &&
        arguments == other.arguments &&
        data == other.data &&
        name == other.name &&
        result == other.result &&
        status == other.status &&
        toolCallId == other.toolCallId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, app.hashCode);
    _$hash = $jc(_$hash, arguments.hashCode);
    _$hash = $jc(_$hash, data.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, result.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, toolCallId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ToolCallSummary')
          ..add('app', app)
          ..add('arguments', arguments)
          ..add('data', data)
          ..add('name', name)
          ..add('result', result)
          ..add('status', status)
          ..add('toolCallId', toolCallId))
        .toString();
  }
}

class ToolCallSummaryBuilder
    implements Builder<ToolCallSummary, ToolCallSummaryBuilder> {
  _$ToolCallSummary? _$v;

  McpAppViewBuilder? _app;
  McpAppViewBuilder get app => _$this._app ??= McpAppViewBuilder();
  set app(McpAppViewBuilder? app) => _$this._app = app;

  JsonObject? _arguments;
  JsonObject? get arguments => _$this._arguments;
  set arguments(JsonObject? arguments) => _$this._arguments = arguments;

  JsonObject? _data;
  JsonObject? get data => _$this._data;
  set data(JsonObject? data) => _$this._data = data;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _result;
  String? get result => _$this._result;
  set result(String? result) => _$this._result = result;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _toolCallId;
  String? get toolCallId => _$this._toolCallId;
  set toolCallId(String? toolCallId) => _$this._toolCallId = toolCallId;

  ToolCallSummaryBuilder() {
    ToolCallSummary._defaults(this);
  }

  ToolCallSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _app = $v.app?.toBuilder();
      _arguments = $v.arguments;
      _data = $v.data;
      _name = $v.name;
      _result = $v.result;
      _status = $v.status;
      _toolCallId = $v.toolCallId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ToolCallSummary other) {
    _$v = other as _$ToolCallSummary;
  }

  @override
  void update(void Function(ToolCallSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ToolCallSummary build() => _build();

  _$ToolCallSummary _build() {
    _$ToolCallSummary _$result;
    try {
      _$result = _$v ??
          _$ToolCallSummary._(
            app: _app?.build(),
            arguments: arguments,
            data: data,
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'ToolCallSummary', 'name'),
            result: result,
            status: BuiltValueNullFieldError.checkNotNull(
                status, r'ToolCallSummary', 'status'),
            toolCallId: toolCallId,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'app';
        _app?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ToolCallSummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
