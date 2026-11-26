// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_search_match.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageSearchMatch extends MessageSearchMatch {
  @override
  final DateTime createdAt;
  @override
  final String messageId;
  @override
  final String role;
  @override
  final BuiltList<SearchSegment> segments;

  factory _$MessageSearchMatch(
          [void Function(MessageSearchMatchBuilder)? updates]) =>
      (MessageSearchMatchBuilder()..update(updates))._build();

  _$MessageSearchMatch._(
      {required this.createdAt,
      required this.messageId,
      required this.role,
      required this.segments})
      : super._();
  @override
  MessageSearchMatch rebuild(
          void Function(MessageSearchMatchBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageSearchMatchBuilder toBuilder() =>
      MessageSearchMatchBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MessageSearchMatch &&
        createdAt == other.createdAt &&
        messageId == other.messageId &&
        role == other.role &&
        segments == other.segments;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, segments.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageSearchMatch')
          ..add('createdAt', createdAt)
          ..add('messageId', messageId)
          ..add('role', role)
          ..add('segments', segments))
        .toString();
  }
}

class MessageSearchMatchBuilder
    implements Builder<MessageSearchMatch, MessageSearchMatchBuilder> {
  _$MessageSearchMatch? _$v;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  ListBuilder<SearchSegment>? _segments;
  ListBuilder<SearchSegment> get segments =>
      _$this._segments ??= ListBuilder<SearchSegment>();
  set segments(ListBuilder<SearchSegment>? segments) =>
      _$this._segments = segments;

  MessageSearchMatchBuilder() {
    MessageSearchMatch._defaults(this);
  }

  MessageSearchMatchBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _messageId = $v.messageId;
      _role = $v.role;
      _segments = $v.segments.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MessageSearchMatch other) {
    _$v = other as _$MessageSearchMatch;
  }

  @override
  void update(void Function(MessageSearchMatchBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MessageSearchMatch build() => _build();

  _$MessageSearchMatch _build() {
    _$MessageSearchMatch _$result;
    try {
      _$result = _$v ??
          _$MessageSearchMatch._(
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'MessageSearchMatch', 'createdAt'),
            messageId: BuiltValueNullFieldError.checkNotNull(
                messageId, r'MessageSearchMatch', 'messageId'),
            role: BuiltValueNullFieldError.checkNotNull(
                role, r'MessageSearchMatch', 'role'),
            segments: segments.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'segments';
        segments.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'MessageSearchMatch', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
