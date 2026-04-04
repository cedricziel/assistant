// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'send_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SendMessageRequest extends SendMessageRequest {
  @override
  final SendMessageConfiguration? configuration;
  @override
  final Message message;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final String? tenant;

  factory _$SendMessageRequest(
          [void Function(SendMessageRequestBuilder)? updates]) =>
      (SendMessageRequestBuilder()..update(updates))._build();

  _$SendMessageRequest._(
      {this.configuration, required this.message, this.metadata, this.tenant})
      : super._();
  @override
  SendMessageRequest rebuild(
          void Function(SendMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SendMessageRequestBuilder toBuilder() =>
      SendMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SendMessageRequest &&
        configuration == other.configuration &&
        message == other.message &&
        metadata == other.metadata &&
        tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, configuration.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SendMessageRequest')
          ..add('configuration', configuration)
          ..add('message', message)
          ..add('metadata', metadata)
          ..add('tenant', tenant))
        .toString();
  }
}

class SendMessageRequestBuilder
    implements Builder<SendMessageRequest, SendMessageRequestBuilder> {
  _$SendMessageRequest? _$v;

  SendMessageConfigurationBuilder? _configuration;
  SendMessageConfigurationBuilder get configuration =>
      _$this._configuration ??= SendMessageConfigurationBuilder();
  set configuration(SendMessageConfigurationBuilder? configuration) =>
      _$this._configuration = configuration;

  MessageBuilder? _message;
  MessageBuilder get message => _$this._message ??= MessageBuilder();
  set message(MessageBuilder? message) => _$this._message = message;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  SendMessageRequestBuilder() {
    SendMessageRequest._defaults(this);
  }

  SendMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _configuration = $v.configuration?.toBuilder();
      _message = $v.message.toBuilder();
      _metadata = $v.metadata?.toBuilder();
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SendMessageRequest other) {
    _$v = other as _$SendMessageRequest;
  }

  @override
  void update(void Function(SendMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SendMessageRequest build() => _build();

  _$SendMessageRequest _build() {
    _$SendMessageRequest _$result;
    try {
      _$result = _$v ??
          _$SendMessageRequest._(
            configuration: _configuration?.build(),
            message: message.build(),
            metadata: _metadata?.build(),
            tenant: tenant,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'configuration';
        _configuration?.build();
        _$failedField = 'message';
        message.build();
        _$failedField = 'metadata';
        _metadata?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SendMessageRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
