// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_capabilities.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentCapabilities extends AgentCapabilities {
  @override
  final bool? extendedAgentCard;
  @override
  final BuiltList<AgentExtension>? extensions;
  @override
  final bool? pushNotifications;
  @override
  final bool? streaming;

  factory _$AgentCapabilities(
          [void Function(AgentCapabilitiesBuilder)? updates]) =>
      (AgentCapabilitiesBuilder()..update(updates))._build();

  _$AgentCapabilities._(
      {this.extendedAgentCard,
      this.extensions,
      this.pushNotifications,
      this.streaming})
      : super._();
  @override
  AgentCapabilities rebuild(void Function(AgentCapabilitiesBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentCapabilitiesBuilder toBuilder() =>
      AgentCapabilitiesBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentCapabilities &&
        extendedAgentCard == other.extendedAgentCard &&
        extensions == other.extensions &&
        pushNotifications == other.pushNotifications &&
        streaming == other.streaming;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, extendedAgentCard.hashCode);
    _$hash = $jc(_$hash, extensions.hashCode);
    _$hash = $jc(_$hash, pushNotifications.hashCode);
    _$hash = $jc(_$hash, streaming.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentCapabilities')
          ..add('extendedAgentCard', extendedAgentCard)
          ..add('extensions', extensions)
          ..add('pushNotifications', pushNotifications)
          ..add('streaming', streaming))
        .toString();
  }
}

class AgentCapabilitiesBuilder
    implements Builder<AgentCapabilities, AgentCapabilitiesBuilder> {
  _$AgentCapabilities? _$v;

  bool? _extendedAgentCard;
  bool? get extendedAgentCard => _$this._extendedAgentCard;
  set extendedAgentCard(bool? extendedAgentCard) =>
      _$this._extendedAgentCard = extendedAgentCard;

  ListBuilder<AgentExtension>? _extensions;
  ListBuilder<AgentExtension> get extensions =>
      _$this._extensions ??= ListBuilder<AgentExtension>();
  set extensions(ListBuilder<AgentExtension>? extensions) =>
      _$this._extensions = extensions;

  bool? _pushNotifications;
  bool? get pushNotifications => _$this._pushNotifications;
  set pushNotifications(bool? pushNotifications) =>
      _$this._pushNotifications = pushNotifications;

  bool? _streaming;
  bool? get streaming => _$this._streaming;
  set streaming(bool? streaming) => _$this._streaming = streaming;

  AgentCapabilitiesBuilder() {
    AgentCapabilities._defaults(this);
  }

  AgentCapabilitiesBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _extendedAgentCard = $v.extendedAgentCard;
      _extensions = $v.extensions?.toBuilder();
      _pushNotifications = $v.pushNotifications;
      _streaming = $v.streaming;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentCapabilities other) {
    _$v = other as _$AgentCapabilities;
  }

  @override
  void update(void Function(AgentCapabilitiesBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentCapabilities build() => _build();

  _$AgentCapabilities _build() {
    _$AgentCapabilities _$result;
    try {
      _$result = _$v ??
          _$AgentCapabilities._(
            extendedAgentCard: extendedAgentCard,
            extensions: _extensions?.build(),
            pushNotifications: pushNotifications,
            streaming: streaming,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'extensions';
        _extensions?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'AgentCapabilities', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
