// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'send_message_configuration.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SendMessageConfiguration extends SendMessageConfiguration {
  @override
  final BuiltList<String>? acceptedOutputModes;
  @override
  final bool? blocking;
  @override
  final int? historyLength;
  @override
  final PushNotificationConfig? pushNotificationConfig;

  factory _$SendMessageConfiguration(
          [void Function(SendMessageConfigurationBuilder)? updates]) =>
      (SendMessageConfigurationBuilder()..update(updates))._build();

  _$SendMessageConfiguration._(
      {this.acceptedOutputModes,
      this.blocking,
      this.historyLength,
      this.pushNotificationConfig})
      : super._();
  @override
  SendMessageConfiguration rebuild(
          void Function(SendMessageConfigurationBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SendMessageConfigurationBuilder toBuilder() =>
      SendMessageConfigurationBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SendMessageConfiguration &&
        acceptedOutputModes == other.acceptedOutputModes &&
        blocking == other.blocking &&
        historyLength == other.historyLength &&
        pushNotificationConfig == other.pushNotificationConfig;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, acceptedOutputModes.hashCode);
    _$hash = $jc(_$hash, blocking.hashCode);
    _$hash = $jc(_$hash, historyLength.hashCode);
    _$hash = $jc(_$hash, pushNotificationConfig.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SendMessageConfiguration')
          ..add('acceptedOutputModes', acceptedOutputModes)
          ..add('blocking', blocking)
          ..add('historyLength', historyLength)
          ..add('pushNotificationConfig', pushNotificationConfig))
        .toString();
  }
}

class SendMessageConfigurationBuilder
    implements
        Builder<SendMessageConfiguration, SendMessageConfigurationBuilder> {
  _$SendMessageConfiguration? _$v;

  ListBuilder<String>? _acceptedOutputModes;
  ListBuilder<String> get acceptedOutputModes =>
      _$this._acceptedOutputModes ??= ListBuilder<String>();
  set acceptedOutputModes(ListBuilder<String>? acceptedOutputModes) =>
      _$this._acceptedOutputModes = acceptedOutputModes;

  bool? _blocking;
  bool? get blocking => _$this._blocking;
  set blocking(bool? blocking) => _$this._blocking = blocking;

  int? _historyLength;
  int? get historyLength => _$this._historyLength;
  set historyLength(int? historyLength) =>
      _$this._historyLength = historyLength;

  PushNotificationConfigBuilder? _pushNotificationConfig;
  PushNotificationConfigBuilder get pushNotificationConfig =>
      _$this._pushNotificationConfig ??= PushNotificationConfigBuilder();
  set pushNotificationConfig(
          PushNotificationConfigBuilder? pushNotificationConfig) =>
      _$this._pushNotificationConfig = pushNotificationConfig;

  SendMessageConfigurationBuilder() {
    SendMessageConfiguration._defaults(this);
  }

  SendMessageConfigurationBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _acceptedOutputModes = $v.acceptedOutputModes?.toBuilder();
      _blocking = $v.blocking;
      _historyLength = $v.historyLength;
      _pushNotificationConfig = $v.pushNotificationConfig?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SendMessageConfiguration other) {
    _$v = other as _$SendMessageConfiguration;
  }

  @override
  void update(void Function(SendMessageConfigurationBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SendMessageConfiguration build() => _build();

  _$SendMessageConfiguration _build() {
    _$SendMessageConfiguration _$result;
    try {
      _$result = _$v ??
          _$SendMessageConfiguration._(
            acceptedOutputModes: _acceptedOutputModes?.build(),
            blocking: blocking,
            historyLength: historyLength,
            pushNotificationConfig: _pushNotificationConfig?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'acceptedOutputModes';
        _acceptedOutputModes?.build();

        _$failedField = 'pushNotificationConfig';
        _pushNotificationConfig?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SendMessageConfiguration', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
