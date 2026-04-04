// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'model_part.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ModelPart extends ModelPart {
  @override
  final JsonObject? data;
  @override
  final String? filename;
  @override
  final String? mediaType;
  @override
  final BuiltMap<String, JsonObject?>? metadata;
  @override
  final String? raw;
  @override
  final String? text;
  @override
  final String? url;

  factory _$ModelPart([void Function(ModelPartBuilder)? updates]) =>
      (ModelPartBuilder()..update(updates))._build();

  _$ModelPart._(
      {this.data,
      this.filename,
      this.mediaType,
      this.metadata,
      this.raw,
      this.text,
      this.url})
      : super._();
  @override
  ModelPart rebuild(void Function(ModelPartBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ModelPartBuilder toBuilder() => ModelPartBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ModelPart &&
        data == other.data &&
        filename == other.filename &&
        mediaType == other.mediaType &&
        metadata == other.metadata &&
        raw == other.raw &&
        text == other.text &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, data.hashCode);
    _$hash = $jc(_$hash, filename.hashCode);
    _$hash = $jc(_$hash, mediaType.hashCode);
    _$hash = $jc(_$hash, metadata.hashCode);
    _$hash = $jc(_$hash, raw.hashCode);
    _$hash = $jc(_$hash, text.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ModelPart')
          ..add('data', data)
          ..add('filename', filename)
          ..add('mediaType', mediaType)
          ..add('metadata', metadata)
          ..add('raw', raw)
          ..add('text', text)
          ..add('url', url))
        .toString();
  }
}

class ModelPartBuilder implements Builder<ModelPart, ModelPartBuilder> {
  _$ModelPart? _$v;

  JsonObject? _data;
  JsonObject? get data => _$this._data;
  set data(JsonObject? data) => _$this._data = data;

  String? _filename;
  String? get filename => _$this._filename;
  set filename(String? filename) => _$this._filename = filename;

  String? _mediaType;
  String? get mediaType => _$this._mediaType;
  set mediaType(String? mediaType) => _$this._mediaType = mediaType;

  MapBuilder<String, JsonObject?>? _metadata;
  MapBuilder<String, JsonObject?> get metadata =>
      _$this._metadata ??= MapBuilder<String, JsonObject?>();
  set metadata(MapBuilder<String, JsonObject?>? metadata) =>
      _$this._metadata = metadata;

  String? _raw;
  String? get raw => _$this._raw;
  set raw(String? raw) => _$this._raw = raw;

  String? _text;
  String? get text => _$this._text;
  set text(String? text) => _$this._text = text;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  ModelPartBuilder() {
    ModelPart._defaults(this);
  }

  ModelPartBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _data = $v.data;
      _filename = $v.filename;
      _mediaType = $v.mediaType;
      _metadata = $v.metadata?.toBuilder();
      _raw = $v.raw;
      _text = $v.text;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ModelPart other) {
    _$v = other as _$ModelPart;
  }

  @override
  void update(void Function(ModelPartBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ModelPart build() => _build();

  _$ModelPart _build() {
    _$ModelPart _$result;
    try {
      _$result = _$v ??
          _$ModelPart._(
            data: data,
            filename: filename,
            mediaType: mediaType,
            metadata: _metadata?.build(),
            raw: raw,
            text: text,
            url: url,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'metadata';
        _metadata?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ModelPart', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
