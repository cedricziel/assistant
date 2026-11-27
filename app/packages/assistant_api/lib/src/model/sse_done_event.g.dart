// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sse_done_event.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SseDoneEvent extends SseDoneEvent {
  @override
  final BuiltList<String>? attachmentIds;
  @override
  final String content;
  @override
  final String? messageId;
  @override
  final String role;
  @override
  final MessageTimingSummary? timing;

  factory _$SseDoneEvent([void Function(SseDoneEventBuilder)? updates]) =>
      (SseDoneEventBuilder()..update(updates))._build();

  _$SseDoneEvent._(
      {this.attachmentIds,
      required this.content,
      this.messageId,
      required this.role,
      this.timing})
      : super._();
  @override
  SseDoneEvent rebuild(void Function(SseDoneEventBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SseDoneEventBuilder toBuilder() => SseDoneEventBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SseDoneEvent &&
        attachmentIds == other.attachmentIds &&
        content == other.content &&
        messageId == other.messageId &&
        role == other.role &&
        timing == other.timing;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, attachmentIds.hashCode);
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, timing.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SseDoneEvent')
          ..add('attachmentIds', attachmentIds)
          ..add('content', content)
          ..add('messageId', messageId)
          ..add('role', role)
          ..add('timing', timing))
        .toString();
  }
}

class SseDoneEventBuilder
    implements Builder<SseDoneEvent, SseDoneEventBuilder> {
  _$SseDoneEvent? _$v;

  ListBuilder<String>? _attachmentIds;
  ListBuilder<String> get attachmentIds =>
      _$this._attachmentIds ??= ListBuilder<String>();
  set attachmentIds(ListBuilder<String>? attachmentIds) =>
      _$this._attachmentIds = attachmentIds;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  MessageTimingSummaryBuilder? _timing;
  MessageTimingSummaryBuilder get timing =>
      _$this._timing ??= MessageTimingSummaryBuilder();
  set timing(MessageTimingSummaryBuilder? timing) => _$this._timing = timing;

  SseDoneEventBuilder() {
    SseDoneEvent._defaults(this);
  }

  SseDoneEventBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _attachmentIds = $v.attachmentIds?.toBuilder();
      _content = $v.content;
      _messageId = $v.messageId;
      _role = $v.role;
      _timing = $v.timing?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SseDoneEvent other) {
    _$v = other as _$SseDoneEvent;
  }

  @override
  void update(void Function(SseDoneEventBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SseDoneEvent build() => _build();

  _$SseDoneEvent _build() {
    _$SseDoneEvent _$result;
    try {
      _$result = _$v ??
          _$SseDoneEvent._(
            attachmentIds: _attachmentIds?.build(),
            content: BuiltValueNullFieldError.checkNotNull(
                content, r'SseDoneEvent', 'content'),
            messageId: messageId,
            role: BuiltValueNullFieldError.checkNotNull(
                role, r'SseDoneEvent', 'role'),
            timing: _timing?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'attachmentIds';
        _attachmentIds?.build();

        _$failedField = 'timing';
        _timing?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SseDoneEvent', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
