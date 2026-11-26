// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'call_mcp_app_tool_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CallMcpAppToolRequest extends CallMcpAppToolRequest {
  @override
  final JsonObject? arguments;
  @override
  final bool? confirmed;
  @override
  final String name;

  factory _$CallMcpAppToolRequest(
          [void Function(CallMcpAppToolRequestBuilder)? updates]) =>
      (CallMcpAppToolRequestBuilder()..update(updates))._build();

  _$CallMcpAppToolRequest._(
      {this.arguments, this.confirmed, required this.name})
      : super._();
  @override
  CallMcpAppToolRequest rebuild(
          void Function(CallMcpAppToolRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CallMcpAppToolRequestBuilder toBuilder() =>
      CallMcpAppToolRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CallMcpAppToolRequest &&
        arguments == other.arguments &&
        confirmed == other.confirmed &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, arguments.hashCode);
    _$hash = $jc(_$hash, confirmed.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CallMcpAppToolRequest')
          ..add('arguments', arguments)
          ..add('confirmed', confirmed)
          ..add('name', name))
        .toString();
  }
}

class CallMcpAppToolRequestBuilder
    implements Builder<CallMcpAppToolRequest, CallMcpAppToolRequestBuilder> {
  _$CallMcpAppToolRequest? _$v;

  JsonObject? _arguments;
  JsonObject? get arguments => _$this._arguments;
  set arguments(JsonObject? arguments) => _$this._arguments = arguments;

  bool? _confirmed;
  bool? get confirmed => _$this._confirmed;
  set confirmed(bool? confirmed) => _$this._confirmed = confirmed;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CallMcpAppToolRequestBuilder() {
    CallMcpAppToolRequest._defaults(this);
  }

  CallMcpAppToolRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _arguments = $v.arguments;
      _confirmed = $v.confirmed;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CallMcpAppToolRequest other) {
    _$v = other as _$CallMcpAppToolRequest;
  }

  @override
  void update(void Function(CallMcpAppToolRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CallMcpAppToolRequest build() => _build();

  _$CallMcpAppToolRequest _build() {
    final _$result = _$v ??
        _$CallMcpAppToolRequest._(
          arguments: arguments,
          confirmed: confirmed,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CallMcpAppToolRequest', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
