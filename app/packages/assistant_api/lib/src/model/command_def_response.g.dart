// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'command_def_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CommandDefResponse extends CommandDefResponse {
  @override
  final BuiltList<CommandArgResponse> args;
  @override
  final String category;
  @override
  final String description;
  @override
  final String name;

  factory _$CommandDefResponse(
          [void Function(CommandDefResponseBuilder)? updates]) =>
      (CommandDefResponseBuilder()..update(updates))._build();

  _$CommandDefResponse._(
      {required this.args,
      required this.category,
      required this.description,
      required this.name})
      : super._();
  @override
  CommandDefResponse rebuild(
          void Function(CommandDefResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CommandDefResponseBuilder toBuilder() =>
      CommandDefResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CommandDefResponse &&
        args == other.args &&
        category == other.category &&
        description == other.description &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, args.hashCode);
    _$hash = $jc(_$hash, category.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CommandDefResponse')
          ..add('args', args)
          ..add('category', category)
          ..add('description', description)
          ..add('name', name))
        .toString();
  }
}

class CommandDefResponseBuilder
    implements Builder<CommandDefResponse, CommandDefResponseBuilder> {
  _$CommandDefResponse? _$v;

  ListBuilder<CommandArgResponse>? _args;
  ListBuilder<CommandArgResponse> get args =>
      _$this._args ??= ListBuilder<CommandArgResponse>();
  set args(ListBuilder<CommandArgResponse>? args) => _$this._args = args;

  String? _category;
  String? get category => _$this._category;
  set category(String? category) => _$this._category = category;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CommandDefResponseBuilder() {
    CommandDefResponse._defaults(this);
  }

  CommandDefResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _args = $v.args.toBuilder();
      _category = $v.category;
      _description = $v.description;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CommandDefResponse other) {
    _$v = other as _$CommandDefResponse;
  }

  @override
  void update(void Function(CommandDefResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CommandDefResponse build() => _build();

  _$CommandDefResponse _build() {
    _$CommandDefResponse _$result;
    try {
      _$result = _$v ??
          _$CommandDefResponse._(
            args: args.build(),
            category: BuiltValueNullFieldError.checkNotNull(
                category, r'CommandDefResponse', 'category'),
            description: BuiltValueNullFieldError.checkNotNull(
                description, r'CommandDefResponse', 'description'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'CommandDefResponse', 'name'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'args';
        args.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CommandDefResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
