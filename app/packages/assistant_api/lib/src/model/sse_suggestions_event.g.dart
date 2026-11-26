// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_suggestions_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseSuggestionsEvent extends SseSuggestionsEvent {
  @override
  final String messageId;
  @override
  final BuiltList<SseSuggestion> suggestions;

  factory _$SseSuggestionsEvent(
          [void Function(SseSuggestionsEventBuilder)? updates]) =>
      (SseSuggestionsEventBuilder()..update(updates))._build();

  _$SseSuggestionsEvent._({required this.messageId, required this.suggestions})
      : super._();
  @override
  SseSuggestionsEvent rebuild(
          void Function(SseSuggestionsEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseSuggestionsEventBuilder toBuilder() =>
      SseSuggestionsEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseSuggestionsEvent &&
        messageId == other.messageId &&
        suggestions == other.suggestions;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jc(_$hash, suggestions.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseSuggestionsEvent')
          ..add('messageId', messageId)
          ..add('suggestions', suggestions))
        .toString();
  }
}

class SseSuggestionsEventBuilder
    implements Builder<SseSuggestionsEvent, SseSuggestionsEventBuilder> {
  _$SseSuggestionsEvent? _$v;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  ListBuilder<SseSuggestion>? _suggestions;
  ListBuilder<SseSuggestion> get suggestions =>
      _$this._suggestions ??= ListBuilder<SseSuggestion>();
  set suggestions(ListBuilder<SseSuggestion>? suggestions) =>
      _$this._suggestions = suggestions;

  SseSuggestionsEventBuilder() {
    SseSuggestionsEvent._defaults(this);
  }

  SseSuggestionsEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _messageId = $v.messageId;
      _suggestions = $v.suggestions.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseSuggestionsEvent other) {
    _$v = other as _$SseSuggestionsEvent;
  }

  @override
  void update(void Function(SseSuggestionsEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseSuggestionsEvent build() => _build();

  _$SseSuggestionsEvent _build() {
    _$SseSuggestionsEvent _$result;
    try {
      _$result = _$v ??
          _$SseSuggestionsEvent._(
            messageId: BuiltValueNullFieldError.checkNotNull(
                messageId, r'SseSuggestionsEvent', 'messageId'),
            suggestions: suggestions.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'suggestions';
        suggestions.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseSuggestionsEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
