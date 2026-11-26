// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'server_capabilities.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ServerCapabilities extends ServerCapabilities {
  @override
  final UiCapabilities ui;
  @override
  final bool voiceReceive;
  @override
  final bool voiceSend;

  factory _$ServerCapabilities(
          [void Function(ServerCapabilitiesBuilder)? updates]) =>
      (ServerCapabilitiesBuilder()..update(updates))._build();

  _$ServerCapabilities._(
      {required this.ui, required this.voiceReceive, required this.voiceSend})
      : super._();
  @override
  ServerCapabilities rebuild(
          void Function(ServerCapabilitiesBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ServerCapabilitiesBuilder toBuilder() =>
      ServerCapabilitiesBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ServerCapabilities &&
        ui == other.ui &&
        voiceReceive == other.voiceReceive &&
        voiceSend == other.voiceSend;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, ui.hashCode);
    _$hash = $jc(_$hash, voiceReceive.hashCode);
    _$hash = $jc(_$hash, voiceSend.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ServerCapabilities')
          ..add('ui', ui)
          ..add('voiceReceive', voiceReceive)
          ..add('voiceSend', voiceSend))
        .toString();
  }
}

class ServerCapabilitiesBuilder
    implements Builder<ServerCapabilities, ServerCapabilitiesBuilder> {
  _$ServerCapabilities? _$v;

  UiCapabilitiesBuilder? _ui;
  UiCapabilitiesBuilder get ui => _$this._ui ??= UiCapabilitiesBuilder();
  set ui(UiCapabilitiesBuilder? ui) => _$this._ui = ui;

  bool? _voiceReceive;
  bool? get voiceReceive => _$this._voiceReceive;
  set voiceReceive(bool? voiceReceive) => _$this._voiceReceive = voiceReceive;

  bool? _voiceSend;
  bool? get voiceSend => _$this._voiceSend;
  set voiceSend(bool? voiceSend) => _$this._voiceSend = voiceSend;

  ServerCapabilitiesBuilder() {
    ServerCapabilities._defaults(this);
  }

  ServerCapabilitiesBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _ui = $v.ui.toBuilder();
      _voiceReceive = $v.voiceReceive;
      _voiceSend = $v.voiceSend;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ServerCapabilities other) {
    _$v = other as _$ServerCapabilities;
  }

  @override
  void update(void Function(ServerCapabilitiesBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ServerCapabilities build() => _build();

  _$ServerCapabilities _build() {
    _$ServerCapabilities _$result;
    try {
      _$result = _$v ??
          _$ServerCapabilities._(
            ui: ui.build(),
            voiceReceive: BuiltValueNullFieldError.checkNotNull(
                voiceReceive, r'ServerCapabilities', 'voiceReceive'),
            voiceSend: BuiltValueNullFieldError.checkNotNull(
                voiceSend, r'ServerCapabilities', 'voiceSend'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'ui';
        ui.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ServerCapabilities', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
