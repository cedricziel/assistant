// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'push_notification_config.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PushNotificationConfig extends PushNotificationConfig {
  @override
  final AuthenticationInfo? authentication;
  @override
  final String? id;
  @override
  final String? token;
  @override
  final String url;

  factory _$PushNotificationConfig(
          [void Function(PushNotificationConfigBuilder)? updates]) =>
      (PushNotificationConfigBuilder()..update(updates))._build();

  _$PushNotificationConfig._(
      {this.authentication, this.id, this.token, required this.url})
      : super._();
  @override
  PushNotificationConfig rebuild(
          void Function(PushNotificationConfigBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PushNotificationConfigBuilder toBuilder() =>
      PushNotificationConfigBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PushNotificationConfig &&
        authentication == other.authentication &&
        id == other.id &&
        token == other.token &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, authentication.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, token.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PushNotificationConfig')
          ..add('authentication', authentication)
          ..add('id', id)
          ..add('token', token)
          ..add('url', url))
        .toString();
  }
}

class PushNotificationConfigBuilder
    implements Builder<PushNotificationConfig, PushNotificationConfigBuilder> {
  _$PushNotificationConfig? _$v;

  AuthenticationInfoBuilder? _authentication;
  AuthenticationInfoBuilder get authentication =>
      _$this._authentication ??= AuthenticationInfoBuilder();
  set authentication(AuthenticationInfoBuilder? authentication) =>
      _$this._authentication = authentication;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _token;
  String? get token => _$this._token;
  set token(String? token) => _$this._token = token;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  PushNotificationConfigBuilder() {
    PushNotificationConfig._defaults(this);
  }

  PushNotificationConfigBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _authentication = $v.authentication?.toBuilder();
      _id = $v.id;
      _token = $v.token;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PushNotificationConfig other) {
    _$v = other as _$PushNotificationConfig;
  }

  @override
  void update(void Function(PushNotificationConfigBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PushNotificationConfig build() => _build();

  _$PushNotificationConfig _build() {
    _$PushNotificationConfig _$result;
    try {
      _$result = _$v ??
          _$PushNotificationConfig._(
            authentication: _authentication?.build(),
            id: id,
            token: token,
            url: BuiltValueNullFieldError.checkNotNull(
                url, r'PushNotificationConfig', 'url'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'authentication';
        _authentication?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'PushNotificationConfig', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
