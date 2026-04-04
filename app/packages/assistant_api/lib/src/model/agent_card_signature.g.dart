// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_card_signature.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentCardSignature extends AgentCardSignature {
  @override
  final BuiltMap<String, JsonObject?>? header;
  @override
  final String protected;
  @override
  final String signature;

  factory _$AgentCardSignature(
          [void Function(AgentCardSignatureBuilder)? updates]) =>
      (AgentCardSignatureBuilder()..update(updates))._build();

  _$AgentCardSignature._(
      {this.header, required this.protected, required this.signature})
      : super._();
  @override
  AgentCardSignature rebuild(
          void Function(AgentCardSignatureBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentCardSignatureBuilder toBuilder() =>
      AgentCardSignatureBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentCardSignature &&
        header == other.header &&
        protected == other.protected &&
        signature == other.signature;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, header.hashCode);
    _$hash = $jc(_$hash, protected.hashCode);
    _$hash = $jc(_$hash, signature.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentCardSignature')
          ..add('header', header)
          ..add('protected', protected)
          ..add('signature', signature))
        .toString();
  }
}

class AgentCardSignatureBuilder
    implements Builder<AgentCardSignature, AgentCardSignatureBuilder> {
  _$AgentCardSignature? _$v;

  MapBuilder<String, JsonObject?>? _header;
  MapBuilder<String, JsonObject?> get header =>
      _$this._header ??= MapBuilder<String, JsonObject?>();
  set header(MapBuilder<String, JsonObject?>? header) =>
      _$this._header = header;

  String? _protected;
  String? get protected => _$this._protected;
  set protected(String? protected) => _$this._protected = protected;

  String? _signature;
  String? get signature => _$this._signature;
  set signature(String? signature) => _$this._signature = signature;

  AgentCardSignatureBuilder() {
    AgentCardSignature._defaults(this);
  }

  AgentCardSignatureBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _header = $v.header?.toBuilder();
      _protected = $v.protected;
      _signature = $v.signature;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentCardSignature other) {
    _$v = other as _$AgentCardSignature;
  }

  @override
  void update(void Function(AgentCardSignatureBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentCardSignature build() => _build();

  _$AgentCardSignature _build() {
    _$AgentCardSignature _$result;
    try {
      _$result = _$v ??
          _$AgentCardSignature._(
            header: _header?.build(),
            protected: BuiltValueNullFieldError.checkNotNull(
                protected, r'AgentCardSignature', 'protected'),
            signature: BuiltValueNullFieldError.checkNotNull(
                signature, r'AgentCardSignature', 'signature'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'header';
        _header?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AgentCardSignature', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
