// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_extension.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentExtension extends AgentExtension {
  @override
  final String? description;
  @override
  final BuiltMap<String, JsonObject?>? params;
  @override
  final bool? required_;
  @override
  final String uri;

  factory _$AgentExtension([void Function(AgentExtensionBuilder)? updates]) =>
      (AgentExtensionBuilder()..update(updates))._build();

  _$AgentExtension._(
      {this.description, this.params, this.required_, required this.uri})
      : super._();
  @override
  AgentExtension rebuild(void Function(AgentExtensionBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentExtensionBuilder toBuilder() => AgentExtensionBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentExtension &&
        description == other.description &&
        params == other.params &&
        required_ == other.required_ &&
        uri == other.uri;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, params.hashCode);
    _$hash = $jc(_$hash, required_.hashCode);
    _$hash = $jc(_$hash, uri.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentExtension')
          ..add('description', description)
          ..add('params', params)
          ..add('required_', required_)
          ..add('uri', uri))
        .toString();
  }
}

class AgentExtensionBuilder
    implements Builder<AgentExtension, AgentExtensionBuilder> {
  _$AgentExtension? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  MapBuilder<String, JsonObject?>? _params;
  MapBuilder<String, JsonObject?> get params =>
      _$this._params ??= MapBuilder<String, JsonObject?>();
  set params(MapBuilder<String, JsonObject?>? params) =>
      _$this._params = params;

  bool? _required_;
  bool? get required_ => _$this._required_;
  set required_(bool? required_) => _$this._required_ = required_;

  String? _uri;
  String? get uri => _$this._uri;
  set uri(String? uri) => _$this._uri = uri;

  AgentExtensionBuilder() {
    AgentExtension._defaults(this);
  }

  AgentExtensionBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _params = $v.params?.toBuilder();
      _required_ = $v.required_;
      _uri = $v.uri;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentExtension other) {
    _$v = other as _$AgentExtension;
  }

  @override
  void update(void Function(AgentExtensionBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentExtension build() => _build();

  _$AgentExtension _build() {
    _$AgentExtension _$result;
    try {
      _$result = _$v ??
          _$AgentExtension._(
            description: description,
            params: _params?.build(),
            required_: required_,
            uri: BuiltValueNullFieldError.checkNotNull(
                uri, r'AgentExtension', 'uri'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'params';
        _params?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AgentExtension', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
