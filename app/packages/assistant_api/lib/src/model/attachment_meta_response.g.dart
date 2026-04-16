// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'attachment_meta_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AttachmentMetaResponse extends AttachmentMetaResponse {
  @override
  final DateTime createdAt;
  @override
  final String filename;
  @override
  final String id;
  @override
  final String mimeType;
  @override
  final int sizeBytes;
  @override
  final String url;

  factory _$AttachmentMetaResponse(
          [void Function(AttachmentMetaResponseBuilder)? updates]) =>
      (AttachmentMetaResponseBuilder()..update(updates))._build();

  _$AttachmentMetaResponse._(
      {required this.createdAt,
      required this.filename,
      required this.id,
      required this.mimeType,
      required this.sizeBytes,
      required this.url})
      : super._();
  @override
  AttachmentMetaResponse rebuild(
          void Function(AttachmentMetaResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AttachmentMetaResponseBuilder toBuilder() =>
      AttachmentMetaResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AttachmentMetaResponse &&
        createdAt == other.createdAt &&
        filename == other.filename &&
        id == other.id &&
        mimeType == other.mimeType &&
        sizeBytes == other.sizeBytes &&
        url == other.url;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, filename.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, mimeType.hashCode);
    _$hash = $jc(_$hash, sizeBytes.hashCode);
    _$hash = $jc(_$hash, url.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AttachmentMetaResponse')
          ..add('createdAt', createdAt)
          ..add('filename', filename)
          ..add('id', id)
          ..add('mimeType', mimeType)
          ..add('sizeBytes', sizeBytes)
          ..add('url', url))
        .toString();
  }
}

class AttachmentMetaResponseBuilder
    implements Builder<AttachmentMetaResponse, AttachmentMetaResponseBuilder> {
  _$AttachmentMetaResponse? _$v;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _filename;
  String? get filename => _$this._filename;
  set filename(String? filename) => _$this._filename = filename;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _mimeType;
  String? get mimeType => _$this._mimeType;
  set mimeType(String? mimeType) => _$this._mimeType = mimeType;

  int? _sizeBytes;
  int? get sizeBytes => _$this._sizeBytes;
  set sizeBytes(int? sizeBytes) => _$this._sizeBytes = sizeBytes;

  String? _url;
  String? get url => _$this._url;
  set url(String? url) => _$this._url = url;

  AttachmentMetaResponseBuilder() {
    AttachmentMetaResponse._defaults(this);
  }

  AttachmentMetaResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _filename = $v.filename;
      _id = $v.id;
      _mimeType = $v.mimeType;
      _sizeBytes = $v.sizeBytes;
      _url = $v.url;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AttachmentMetaResponse other) {
    _$v = other as _$AttachmentMetaResponse;
  }

  @override
  void update(void Function(AttachmentMetaResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AttachmentMetaResponse build() => _build();

  _$AttachmentMetaResponse _build() {
    final _$result = _$v ??
        _$AttachmentMetaResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'AttachmentMetaResponse', 'createdAt'),
          filename: BuiltValueNullFieldError.checkNotNull(
              filename, r'AttachmentMetaResponse', 'filename'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'AttachmentMetaResponse', 'id'),
          mimeType: BuiltValueNullFieldError.checkNotNull(
              mimeType, r'AttachmentMetaResponse', 'mimeType'),
          sizeBytes: BuiltValueNullFieldError.checkNotNull(
              sizeBytes, r'AttachmentMetaResponse', 'sizeBytes'),
          url: BuiltValueNullFieldError.checkNotNull(
              url, r'AttachmentMetaResponse', 'url'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
