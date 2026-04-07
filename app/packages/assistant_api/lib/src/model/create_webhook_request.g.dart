// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_webhook_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateWebhookRequest extends CreateWebhookRequest {
  @override
  final bool? active;
  @override
  final BuiltList<String> eventTypes;
  @override
  final String name;
  @override
  final String url;

  factory _$CreateWebhookRequest(
          [void Function(CreateWebhookRequestBuilder)? updates]) =>
      (CreateWebhookRequestBuilder()..update(updates))._build();

  _$CreateWebhookRequest._(
      {this.active,
      required this.eventTypes,
      required this.name,
      required this.url})
      : super._();
  @override
  CreateWebhookRequest rebuild(
          void Function(CreateWebhookRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateWebhookRequestBuilder toBuilder() =>
      CreateWebhookRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateWebhookRequest &&
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
    return (newBuiltValueToStringHelper(r'CreateWebhookRequest')
          ..add('active', active)
          ..add('eventTypes', eventTypes)
          ..add('name', name)
          ..add('url', url))
        .toString();
  }
}

class CreateWebhookRequestBuilder
    implements Builder<CreateWebhookRequest, CreateWebhookRequestBuilder> {
  _$CreateWebhookRequest? _$v;

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

  CreateWebhookRequestBuilder() {
    CreateWebhookRequest._defaults(this);
  }

  CreateWebhookRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _eventTypes = $v.eventTypes.toBuilder();
      _name = $v.name;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateWebhookRequest other) {
    _$v = other as _$CreateWebhookRequest;
  }

  @override
  void update(void Function(CreateWebhookRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateWebhookRequest build() => _build();

  _$CreateWebhookRequest _build() {
    _$CreateWebhookRequest _$result;
    try {
      _$result = _$v ??
          _$CreateWebhookRequest._(
            active: active,
            eventTypes: eventTypes.build(),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'CreateWebhookRequest', 'name'),
            url: BuiltValueNullFieldError.checkNotNull(
                url, r'CreateWebhookRequest', 'url'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'eventTypes';
        eventTypes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CreateWebhookRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
