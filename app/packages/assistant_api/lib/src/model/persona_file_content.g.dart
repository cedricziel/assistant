// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'persona_file_content.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PersonaFileContent extends PersonaFileContent {
  @override
  final String content;
  @override
  final String filename;

  factory _$PersonaFileContent(
          [void Function(PersonaFileContentBuilder)? updates]) =>
      (PersonaFileContentBuilder()..update(updates))._build();

  _$PersonaFileContent._({required this.content, required this.filename})
      : super._();
  @override
  PersonaFileContent rebuild(
          void Function(PersonaFileContentBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  PersonaFileContentBuilder toBuilder() =>
      PersonaFileContentBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PersonaFileContent &&
        content == other.content &&
        filename == other.filename;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, filename.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PersonaFileContent')
          ..add('content', content)
          ..add('filename', filename))
        .toString();
  }
}

class PersonaFileContentBuilder
    implements Builder<PersonaFileContent, PersonaFileContentBuilder> {
  _$PersonaFileContent? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _filename;
  String? get filename => _$this._filename;
  set filename(String? filename) => _$this._filename = filename;

  PersonaFileContentBuilder() {
    PersonaFileContent._defaults(this);
  }

  PersonaFileContentBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _filename = $v.filename;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PersonaFileContent other) {
    _$v = other as _$PersonaFileContent;
  }

  @override
  void update(void Function(PersonaFileContentBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PersonaFileContent build() => _build();

  _$PersonaFileContent _build() {
    final _$result = _$v ??
        _$PersonaFileContent._(
          content: BuiltValueNullFieldError.checkNotNull(
              content, r'PersonaFileContent', 'content'),
          filename: BuiltValueNullFieldError.checkNotNull(
              filename, r'PersonaFileContent', 'filename'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
