// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_webhook_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateWebhookRequest extends UpdateWebhookRequest {
  @override
  final bool? active;
  @override
  final BuiltList<String>? eventTypes;
  @override
  final String? name;
  @override
  final String? url;

  factory _$UpdateWebhookRequest(
          [void Function(UpdateWebhookRequestBuilder)? updates]) =>
      (UpdateWebhookRequestBuilder()..update(updates))._build();

  _$UpdateWebhookRequest._({this.active, this.eventTypes, this.name, this.url})
      : super._();
  @override
  UpdateWebhookRequest rebuild(
          void Function(UpdateWebhookRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateWebhookRequestBuilder toBuilder() =>
      UpdateWebhookRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateWebhookRequest &&
        active == other.active &&
        eventTypes == other.eventTypes &&
        name == other.name &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, eventTypes.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateWebhookRequest')
          ..add('active', active)
          ..add('eventTypes', eventTypes)
          ..add('name', name)
          ..add('url', url))
        .toString();
  }
}

class UpdateWebhookRequestBuilder
    implements Builder<UpdateWebhookRequest, UpdateWebhookRequestBuilder> {
  _$UpdateWebhookRequest? _$v;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  ListBuilder<String>? _eventTypes;
  ListBuilder<String> get eventTypes =>
      _$this._eventTypes ??= ListBuilder<String>();
  set eventTypes(ListBuilder<String>? eventTypes) =>
      _$this._eventTypes = eventTypes;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  UpdateWebhookRequestBuilder() {
    UpdateWebhookRequest._defaults(this);
  }

  UpdateWebhookRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _eventTypes = $v.eventTypes?.toBuilder();
      _name = $v.name;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateWebhookRequest other) {
    _$v = other as _$UpdateWebhookRequest;
  }

  @override
  void update(void Function(UpdateWebhookRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateWebhookRequest build() => _build();

  _$UpdateWebhookRequest _build() {
    _$UpdateWebhookRequest _$result;
    try {
      _$result = _$v ??
          _$UpdateWebhookRequest._(
            active: active,
            eventTypes: _eventTypes?.build(),
            name: name,
            url: url,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'eventTypes';
        _eventTypes?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'UpdateWebhookRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
