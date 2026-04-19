// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'execute_command_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExecuteCommandRequest extends ExecuteCommandRequest {
  @override
  final BuiltList<String>? args;
  @override
  final String command;

  factory _$ExecuteCommandRequest(
          [void Function(ExecuteCommandRequestBuilder)? updates]) =>
      (ExecuteCommandRequestBuilder()..update(updates))._build();

  _$ExecuteCommandRequest._({this.args, required this.command}) : super._();
  @override
  ExecuteCommandRequest rebuild(
          void Function(ExecuteCommandRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ExecuteCommandRequestBuilder toBuilder() =>
      ExecuteCommandRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExecuteCommandRequest &&
        args == other.args &&
        command == other.command;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, args.hashCode);
    _$hash = $jc(_$hash, command.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExecuteCommandRequest')
          ..add('args', args)
          ..add('command', command))
        .toString();
  }
}

class ExecuteCommandRequestBuilder
    implements Builder<ExecuteCommandRequest, ExecuteCommandRequestBuilder> {
  _$ExecuteCommandRequest? _$v;

  ListBuilder<String>? _args;
  ListBuilder<String> get args => _$this._args ??= ListBuilder<String>();
  set args(ListBuilder<String>? args) => _$this._args = args;

  String? _command;
  String? get command => _$this._command;
  set command(String? command) => _$this._command = command;

  ExecuteCommandRequestBuilder() {
    ExecuteCommandRequest._defaults(this);
  }

  ExecuteCommandRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _args = $v.args?.toBuilder();
      _command = $v.command;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExecuteCommandRequest other) {
    _$v = other as _$ExecuteCommandRequest;
  }

  @override
  void update(void Function(ExecuteCommandRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExecuteCommandRequest build() => _build();

  _$ExecuteCommandRequest _build() {
    _$ExecuteCommandRequest _$result;
    try {
      _$result = _$v ??
          _$ExecuteCommandRequest._(
            args: _args?.build(),
            command: BuiltValueNullFieldError.checkNotNull(
                command, r'ExecuteCommandRequest', 'command'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'args';
        _args?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ExecuteCommandRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
