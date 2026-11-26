// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_suggestion.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSuggestion extends SseSuggestion {
  @override
  final String prompt;

  factory _$SseSuggestion([void Function(SseSuggestionBuilder)? updates]) =>
      (SseSuggestionBuilder()..update(updates))._build();

  _$SseSuggestion._({required this.prompt}) : super._();
  @override
  SseSuggestion rebuild(void Function(SseSuggestionBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSuggestionBuilder toBuilder() => SseSuggestionBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSuggestion && prompt == other.prompt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, prompt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseSuggestion')
          ..add('prompt', prompt))
        .toString();
  }
}

class SseSuggestionBuilder
    implements Builder<SseSuggestion, SseSuggestionBuilder> {
  _$SseSuggestion? _$v;

  String? _prompt;
  String? get prompt => _$this._prompt;
  set prompt(String? prompt) => _$this._prompt = prompt;

  SseSuggestionBuilder() {
    SseSuggestion._defaults(this);
  }

  SseSuggestionBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _prompt = $v.prompt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSuggestion other) {
    _$v = other as _$SseSuggestion;
  }

  @override
  void update(void Function(SseSuggestionBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSuggestion build() => _build();

  _$SseSuggestion _build() {
    final _$result = _$v ??
        _$SseSuggestion._(
          prompt: BuiltValueNullFieldError.checkNotNull(
              prompt, r'SseSuggestion', 'prompt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
