// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_token_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseTokenEvent extends SseTokenEvent {
  @override
  final String content;

  factory _$SseTokenEvent([void Function(SseTokenEventBuilder)? updates]) =>
      (SseTokenEventBuilder()..update(updates))._build();

  _$SseTokenEvent._({required this.content}) : super._();
  @override
  SseTokenEvent rebuild(void Function(SseTokenEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseTokenEventBuilder toBuilder() => SseTokenEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseTokenEvent && content == other.content;
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
    return (newBuiltValueToStringHelper(r'SseTokenEvent')
          ..add('content', content))
        .toString();
  }
}

class SseTokenEventBuilder
    implements Builder<SseTokenEvent, SseTokenEventBuilder> {
  _$SseTokenEvent? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  SseTokenEventBuilder() {
    SseTokenEvent._defaults(this);
  }

  SseTokenEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseTokenEvent other) {
    _$v = other as _$SseTokenEvent;
  }

  @override
  void update(void Function(SseTokenEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseTokenEvent build() => _build();

  _$SseTokenEvent _build() {
    final _$result = _$v ??
        _$SseTokenEvent._(
          content: BuiltValueNullFieldError.checkNotNull(
              content, r'SseTokenEvent', 'content'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
