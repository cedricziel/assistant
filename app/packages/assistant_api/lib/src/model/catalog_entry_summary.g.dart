// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'catalog_entry_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CatalogEntrySummary extends CatalogEntrySummary {
  @override
  final CatalogAuthResponse auth;
  @override
  final BuiltList<String> categories;
  @override
  final String connectUrl;
  @override
  final String description;
  @override
  final String domain;
  @override
  final String? iconUrl;
  @override
  final String id;
  @override
  final String name;

  factory _$CatalogEntrySummary(
          [void Function(CatalogEntrySummaryBuilder)? updates]) =>
      (CatalogEntrySummaryBuilder()..update(updates))._build();

  _$CatalogEntrySummary._(
      {required this.auth,
      required this.categories,
      required this.connectUrl,
      required this.description,
      required this.domain,
      this.iconUrl,
      required this.id,
      required this.name})
      : super._();
  @override
  CatalogEntrySummary rebuild(
          void Function(CatalogEntrySummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CatalogEntrySummaryBuilder toBuilder() =>
      CatalogEntrySummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CatalogEntrySummary &&
        auth == other.auth &&
        categories == other.categories &&
        connectUrl == other.connectUrl &&
        description == other.description &&
        domain == other.domain &&
        iconUrl == other.iconUrl &&
        id == other.id &&
        name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, auth.hashCode);
    _$hash = $jc(_$hash, categories.hashCode);
    _$hash = $jc(_$hash, connectUrl.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, domain.hashCode);
    _$hash = $jc(_$hash, iconUrl.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CatalogEntrySummary')
          ..add('auth', auth)
          ..add('categories', categories)
          ..add('connectUrl', connectUrl)
          ..add('description', description)
          ..add('domain', domain)
          ..add('iconUrl', iconUrl)
          ..add('id', id)
          ..add('name', name))
        .toString();
  }
}

class CatalogEntrySummaryBuilder
    implements Builder<CatalogEntrySummary, CatalogEntrySummaryBuilder> {
  _$CatalogEntrySummary? _$v;

  CatalogAuthResponseBuilder? _auth;
  CatalogAuthResponseBuilder get auth =>
      _$this._auth ??= CatalogAuthResponseBuilder();
  set auth(CatalogAuthResponseBuilder? auth) => _$this._auth = auth;

  ListBuilder<String>? _categories;
  ListBuilder<String> get categories =>
      _$this._categories ??= ListBuilder<String>();
  set categories(ListBuilder<String>? categories) =>
      _$this._categories = categories;

  String? _connectUrl;
  String? get connectUrl => _$this._connectUrl;
  set connectUrl(String? connectUrl) => _$this._connectUrl = connectUrl;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _domain;
  String? get domain => _$this._domain;
  set domain(String? domain) => _$this._domain = domain;

  String? _iconUrl;
  String? get iconUrl => _$this._iconUrl;
  set iconUrl(String? iconUrl) => _$this._iconUrl = iconUrl;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CatalogEntrySummaryBuilder() {
    CatalogEntrySummary._defaults(this);
  }

  CatalogEntrySummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _auth = $v.auth.toBuilder();
      _categories = $v.categories.toBuilder();
      _connectUrl = $v.connectUrl;
      _description = $v.description;
      _domain = $v.domain;
      _iconUrl = $v.iconUrl;
      _id = $v.id;
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CatalogEntrySummary other) {
    _$v = other as _$CatalogEntrySummary;
  }

  @override
  void update(void Function(CatalogEntrySummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CatalogEntrySummary build() => _build();

  _$CatalogEntrySummary _build() {
    _$CatalogEntrySummary _$result;
    try {
      _$result = _$v ??
          _$CatalogEntrySummary._(
            auth: auth.build(),
            categories: categories.build(),
            connectUrl: BuiltValueNullFieldError.checkNotNull(
                connectUrl, r'CatalogEntrySummary', 'connectUrl'),
            description: BuiltValueNullFieldError.checkNotNull(
                description, r'CatalogEntrySummary', 'description'),
            domain: BuiltValueNullFieldError.checkNotNull(
                domain, r'CatalogEntrySummary', 'domain'),
            iconUrl: iconUrl,
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'CatalogEntrySummary', 'id'),
            name: BuiltValueNullFieldError.checkNotNull(
                name, r'CatalogEntrySummary', 'name'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'auth';
        auth.build();
        _$failedField = 'categories';
        categories.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'CatalogEntrySummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
