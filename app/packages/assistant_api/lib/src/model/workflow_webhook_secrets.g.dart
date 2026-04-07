// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'workflow_webhook_secrets.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WorkflowWebhookSecrets extends WorkflowWebhookSecrets {
  @override
  final String webhookToken;
  @override
  final String webhookUrl;

  factory _$WorkflowWebhookSecrets(
          [void Function(WorkflowWebhookSecretsBuilder)? updates]) =>
      (WorkflowWebhookSecretsBuilder()..update(updates))._build();

  _$WorkflowWebhookSecrets._(
      {required this.webhookToken, required this.webhookUrl})
      : super._();
  @override
  WorkflowWebhookSecrets rebuild(
          void Function(WorkflowWebhookSecretsBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WorkflowWebhookSecretsBuilder toBuilder() =>
      WorkflowWebhookSecretsBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WorkflowWebhookSecrets &&
        webhookToken == other.webhookToken &&
        webhookUrl == other.webhookUrl;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, webhookToken.hashCode);
    _$hash = $jc(_$hash, webhookUrl.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WorkflowWebhookSecrets')
          ..add('webhookToken', webhookToken)
          ..add('webhookUrl', webhookUrl))
        .toString();
  }
}

class WorkflowWebhookSecretsBuilder
    implements Builder<WorkflowWebhookSecrets, WorkflowWebhookSecretsBuilder> {
  _$WorkflowWebhookSecrets? _$v;

  String? _webhookToken;
  String? get webhookToken => _$this._webhookToken;
  set webhookToken(String? webhookToken) => _$this._webhookToken = webhookToken;

  String? _webhookUrl;
  String? get webhookUrl => _$this._webhookUrl;
  set webhookUrl(String? webhookUrl) => _$this._webhookUrl = webhookUrl;

  WorkflowWebhookSecretsBuilder() {
    WorkflowWebhookSecrets._defaults(this);
  }

  WorkflowWebhookSecretsBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _webhookToken = $v.webhookToken;
      _webhookUrl = $v.webhookUrl;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WorkflowWebhookSecrets other) {
    _$v = other as _$WorkflowWebhookSecrets;
  }

  @override
  void update(void Function(WorkflowWebhookSecretsBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WorkflowWebhookSecrets build() => _build();

  _$WorkflowWebhookSecrets _build() {
    final _$result = _$v ??
        _$WorkflowWebhookSecrets._(
          webhookToken: BuiltValueNullFieldError.checkNotNull(
              webhookToken, r'WorkflowWebhookSecrets', 'webhookToken'),
          webhookUrl: BuiltValueNullFieldError.checkNotNull(
              webhookUrl, r'WorkflowWebhookSecrets', 'webhookUrl'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
