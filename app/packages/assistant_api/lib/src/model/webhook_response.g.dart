// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'webhook_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WebhookResponse extends WebhookResponse {
  @override
  final bool active;
  @override
  final DateTime createdAt;
  @override
  final BuiltList<String> eventTypes;
  @override
  final String id;
  @override
  final String name;
  @override
  final String secret;
  @override
  final DateTime updatedAt;
  @override
  final String url;
  @override
  final DateTime? verifiedAt;

  factory _$WebhookResponse([void Function(WebhookResponseBuilder)? updates]) =>
      (WebhookResponseBuilder()..update(updates))._build();

  _$WebhookResponse._(
      {required this.active,
      required this.createdAt,
      required this.eventTypes,
      required this.id,
      required this.name,
      required this.secret,
      required this.updatedAt,
      required this.url,
      this.verifiedAt})
      : super._();
  @override
  WebhookResponse rebuild(void Function(WebhookResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WebhookResponseBuilder toBuilder() => WebhookResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WebhookResponse &&
        active == other.active &&
        createdAt == other.createdAt &&
        eventTypes == other.eventTypes &&
        id == other.id &&
        name == other.name &&
        secret == other.secret &&
        updatedAt == other.updatedAt &&
        url == other.url &&
        verifiedAt == other.verifiedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, active.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, eventTypes.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, secret.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jc(_$hash, verifiedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WebhookResponse')
          ..add('active', active)
          ..add('createdAt', createdAt)
          ..add('eventTypes', eventTypes)
          ..add('id', id)
          ..add('name', name)
          ..add('secret', secret)
          ..add('updatedAt', updatedAt)
          ..add('url', url)
          ..add('verifiedAt', verifiedAt))
        .toString();
  }
}

class WebhookResponseBuilder
    implements Builder<WebhookResponse, WebhookResponseBuilder> {
  _$WebhookResponse? _$v;

  bool? _active;
  bool? get active => _$this._active;
  set active(bool? active) => _$this._active = active;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  ListBuilder<String>? _eventTypes;
  ListBuilder<String> get eventTypes =>
      _$this._eventTypes ??= ListBuilder<String>();
  set eventTypes(ListBuilder<String>? eventTypes) =>
      _$this._eventTypes = eventTypes;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _secret;
  String? get secret => _$this._secret;
  set secret(String? secret) => _$this._secret = secret;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  DateTime? _verifiedAt;
  DateTime? get verifiedAt => _$this._verifiedAt;
  set verifiedAt(DateTime? verifiedAt) => _$this._verifiedAt = verifiedAt;

  WebhookResponseBuilder() {
    WebhookResponse._defaults(this);
  }

  WebhookResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _active = $v.active;
      _createdAt = $v.createdAt;
      _eventTypes = $v.eventTypes.toBuilder();
      _id = $v.id;
      _name = $v.name;
      _secret = $v.secret;
      _updatedAt = $v.updatedAt;
      _url = $v.url;
      _verifiedAt = $v.verifiedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WebhookResponse other) {
    _$v = other as _$WebhookResponse;
  }

  @override
  void update(void Function(WebhookResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WebhookResponse build() => _build();

  _$WebhookResponse _build() {
    _$WebhookResponse _$result;
    try {
      _$result = _$v ??
          _$WebhookResponse._(
            active: BuiltValueNullFieldError.checkNotNull(
                active, r'WebhookResponse', 'active'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'WebhookResponse', 'createdAt'),
            eventTypes: eventTypes.build(),
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'WebhookResponse', 'id'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'WebhookResponse', 'name'),
            secret: BuiltValueNullFieldError.checkNotNull(
                secret, r'WebhookResponse', 'secret'),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'WebhookResponse', 'updatedAt'),
            url: BuiltValueNullFieldError.checkNotNull(
                url, r'WebhookResponse', 'url'),
            verifiedAt: verifiedAt,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'eventTypes';
        eventTypes.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'WebhookResponse', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
