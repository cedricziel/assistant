// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'write_persona_file_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$WritePersonaFileRequest extends WritePersonaFileRequest {
  @override
  final String content;

  factory _$WritePersonaFileRequest(
          [void Function(WritePersonaFileRequestBuilder)? updates]) =>
      (WritePersonaFileRequestBuilder()..update(updates))._build();

  _$WritePersonaFileRequest._({required this.content}) : super._();
  @override
  WritePersonaFileRequest rebuild(
          void Function(WritePersonaFileRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  WritePersonaFileRequestBuilder toBuilder() =>
      WritePersonaFileRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is WritePersonaFileRequest && content == other.content;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'WritePersonaFileRequest')
          ..add('content', content))
        .toString();
  }
}

class WritePersonaFileRequestBuilder
    implements
        Builder<WritePersonaFileRequest, WritePersonaFileRequestBuilder> {
  _$WritePersonaFileRequest? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  WritePersonaFileRequestBuilder() {
    WritePersonaFileRequest._defaults(this);
  }

  WritePersonaFileRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(WritePersonaFileRequest other) {
    _$v = other as _$WritePersonaFileRequest;
  }

  @override
  void update(void Function(WritePersonaFileRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  WritePersonaFileRequest build() => _build();

  _$WritePersonaFileRequest _build() {
    final _$result = _$v ??
        _$WritePersonaFileRequest._(
          content: BuiltValueNullFieldError.checkNotNull(
              content, r'WritePersonaFileRequest', 'content'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
