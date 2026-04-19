// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'command_arg_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CommandArgResponse extends CommandArgResponse {
  @override
  final String? completionsEndpoint;
  @override
  final String name;
  @override
  final bool required_;

  factory _$CommandArgResponse(
          [void Function(CommandArgResponseBuilder)? updates]) =>
      (CommandArgResponseBuilder()..update(updates))._build();

  _$CommandArgResponse._(
      {this.completionsEndpoint, required this.name, required this.required_})
      : super._();
  @override
  CommandArgResponse rebuild(
          void Function(CommandArgResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CommandArgResponseBuilder toBuilder() =>
      CommandArgResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CommandArgResponse &&
        completionsEndpoint == other.completionsEndpoint &&
        name == other.name &&
        required_ == other.required_;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, completionsEndpoint.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, required_.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CommandArgResponse')
          ..add('completionsEndpoint', completionsEndpoint)
          ..add('name', name)
          ..add('required_', required_))
        .toString();
  }
}

class CommandArgResponseBuilder
    implements Builder<CommandArgResponse, CommandArgResponseBuilder> {
  _$CommandArgResponse? _$v;

  String? _completionsEndpoint;
  String? get completionsEndpoint => _$this._completionsEndpoint;
  set completionsEndpoint(String? completionsEndpoint) =>
      _$this._completionsEndpoint = completionsEndpoint;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  bool? _required_;
  bool? get required_ => _$this._required_;
  set required_(bool? required_) => _$this._required_ = required_;

  CommandArgResponseBuilder() {
    CommandArgResponse._defaults(this);
  }

  CommandArgResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _completionsEndpoint = $v.completionsEndpoint;
      _name = $v.name;
      _required_ = $v.required_;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CommandArgResponse other) {
    _$v = other as _$CommandArgResponse;
  }

  @override
  void update(void Function(CommandArgResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CommandArgResponse build() => _build();

  _$CommandArgResponse _build() {
    final _$result = _$v ??
        _$CommandArgResponse._(
          completionsEndpoint: completionsEndpoint,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CommandArgResponse', 'name'),
          required_: BuiltValueNullFieldError.checkNotNull(
              required_, r'CommandArgResponse', 'required_'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
