// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'list_task_push_notification_configs_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ListTaskPushNotificationConfigsResponse
    extends ListTaskPushNotificationConfigsResponse {
  @override
  final BuiltList<TaskPushNotificationConfig> configs;
  @override
  final String? nextPageToken;

  factory _$ListTaskPushNotificationConfigsResponse(
          [void Function(ListTaskPushNotificationConfigsResponseBuilder)?
              updates]) =>
      (ListTaskPushNotificationConfigsResponseBuilder()..update(updates))
          ._build();

  _$ListTaskPushNotificationConfigsResponse._(
      {required this.configs, this.nextPageToken})
      : super._();
  @override
  ListTaskPushNotificationConfigsResponse rebuild(
          void Function(ListTaskPushNotificationConfigsResponseBuilder)
              updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ListTaskPushNotificationConfigsResponseBuilder toBuilder() =>
      ListTaskPushNotificationConfigsResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ListTaskPushNotificationConfigsResponse &&
        configs == other.configs &&
        nextPageToken == other.nextPageToken;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, configs.hashCode);
    _$hash = $jc(_$hash, nextPageToken.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
            r'ListTaskPushNotificationConfigsResponse')
          ..add('configs', configs)
          ..add('nextPageToken', nextPageToken))
        .toString();
  }
}

class ListTaskPushNotificationConfigsResponseBuilder
    implements
        Builder<ListTaskPushNotificationConfigsResponse,
            ListTaskPushNotificationConfigsResponseBuilder> {
  _$ListTaskPushNotificationConfigsResponse? _$v;

  ListBuilder<TaskPushNotificationConfig>? _configs;
  ListBuilder<TaskPushNotificationConfig> get configs =>
      _$this._configs ??= ListBuilder<TaskPushNotificationConfig>();
  set configs(ListBuilder<TaskPushNotificationConfig>? configs) =>
      _$this._configs = configs;

  String? _nextPageToken;
  String? get nextPageToken => _$this._nextPageToken;
  set nextPageToken(String? nextPageToken) =>
      _$this._nextPageToken = nextPageToken;

  ListTaskPushNotificationConfigsResponseBuilder() {
    ListTaskPushNotificationConfigsResponse._defaults(this);
  }

  ListTaskPushNotificationConfigsResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _configs = $v.configs.toBuilder();
      _nextPageToken = $v.nextPageToken;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ListTaskPushNotificationConfigsResponse other) {
    _$v = other as _$ListTaskPushNotificationConfigsResponse;
  }

  @override
  void update(
      void Function(ListTaskPushNotificationConfigsResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ListTaskPushNotificationConfigsResponse build() => _build();

  _$ListTaskPushNotificationConfigsResponse _build() {
    _$ListTaskPushNotificationConfigsResponse _$result;
    try {
      _$result = _$v ??
          _$ListTaskPushNotificationConfigsResponse._(
            configs: configs.build(),
            nextPageToken: nextPageToken,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'configs';
        configs.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ListTaskPushNotificationConfigsResponse',
            _$failedField,
            e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
