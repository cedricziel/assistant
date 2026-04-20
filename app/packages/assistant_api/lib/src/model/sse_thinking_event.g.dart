// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_thinking_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseThinkingEvent extends SseThinkingEvent {
  @override
  final String content;

  factory _$SseThinkingEvent(
          [void Function(SseThinkingEventBuilder)? updates]) =>
      (SseThinkingEventBuilder()..update(updates))._build();

  _$SseThinkingEvent._({required this.content}) : super._();
  @override
  SseThinkingEvent rebuild(void Function(SseThinkingEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseThinkingEventBuilder toBuilder() =>
      SseThinkingEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseThinkingEvent && content == other.content;
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
    return (newBuiltValueToStringHelper(r'SseThinkingEvent')
          ..add('content', content))
        .toString();
  }
}

class SseThinkingEventBuilder
    implements Builder<SseThinkingEvent, SseThinkingEventBuilder> {
  _$SseThinkingEvent? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  SseThinkingEventBuilder() {
    SseThinkingEvent._defaults(this);
  }

  SseThinkingEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseThinkingEvent other) {
    _$v = other as _$SseThinkingEvent;
  }

  @override
  void update(void Function(SseThinkingEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseThinkingEvent build() => _build();

  _$SseThinkingEvent _build() {
    final _$result = _$v ??
        _$SseThinkingEvent._(
          content: BuiltValueNullFieldError.checkNotNull(
              content, r'SseThinkingEvent', 'content'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
