// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageSummary extends MessageSummary {
  @override
  final String content;
  @override
  final DateTime createdAt;
  @override
  final String id;
  @override
  final String role;
  @override
  final String? skillName;
  @override
  final BuiltList<String>? toolCalls;
  @override
  final bool ttsAvailable;
  @override
  final int turn;

  factory _$MessageSummary([void Function(MessageSummaryBuilder)? updates]) =>
      (MessageSummaryBuilder()..update(updates))._build();

  _$MessageSummary._(
      {required this.content,
      required this.createdAt,
      required this.id,
      required this.role,
      this.skillName,
      this.toolCalls,
      required this.ttsAvailable,
      required this.turn})
      : super._();
  @override
  MessageSummary rebuild(void Function(MessageSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  MessageSummaryBuilder toBuilder() => MessageSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is MessageSummary &&
        content == other.content &&
        createdAt == other.createdAt &&
        id == other.id &&
        role == other.role &&
        skillName == other.skillName &&
        toolCalls == other.toolCalls &&
        ttsAvailable == other.ttsAvailable &&
        turn == other.turn;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, skillName.hashCode);
    _$hash = $jc(_$hash, toolCalls.hashCode);
    _$hash = $jc(_$hash, ttsAvailable.hashCode);
    _$hash = $jc(_$hash, turn.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageSummary')
          ..add('content', content)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('role', role)
          ..add('skillName', skillName)
          ..add('toolCalls', toolCalls)
          ..add('ttsAvailable', ttsAvailable)
          ..add('turn', turn))
        .toString();
  }
}

class MessageSummaryBuilder
    implements Builder<MessageSummary, MessageSummaryBuilder> {
  _$MessageSummary? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  String? _skillName;
  String? get skillName => _$this._skillName;
  set skillName(String? skillName) => _$this._skillName = skillName;

  ListBuilder<String>? _toolCalls;
  ListBuilder<String> get toolCalls =>
      _$this._toolCalls ??= ListBuilder<String>();
  set toolCalls(ListBuilder<String>? toolCalls) =>
      _$this._toolCalls = toolCalls;

  bool? _ttsAvailable;
  bool? get ttsAvailable => _$this._ttsAvailable;
  set ttsAvailable(bool? ttsAvailable) => _$this._ttsAvailable = ttsAvailable;

  int? _turn;
  int? get turn => _$this._turn;
  set turn(int? turn) => _$this._turn = turn;

  MessageSummaryBuilder() {
    MessageSummary._defaults(this);
  }

  MessageSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _role = $v.role;
      _skillName = $v.skillName;
      _toolCalls = $v.toolCalls?.toBuilder();
      _ttsAvailable = $v.ttsAvailable;
      _turn = $v.turn;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(MessageSummary other) {
    _$v = other as _$MessageSummary;
  }

  @override
  void update(void Function(MessageSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  MessageSummary build() => _build();

  _$MessageSummary _build() {
    _$MessageSummary _$result;
    try {
      _$result = _$v ??
          _$MessageSummary._(
            content: BuiltValueNullFieldError.checkNotNull(
                content, r'MessageSummary', 'content'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'MessageSummary', 'createdAt'),
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'MessageSummary', 'id'),
            role: BuiltValueNullFieldError.checkNotNull(
                role, r'MessageSummary', 'role'),
            skillName: skillName,
            toolCalls: _toolCalls?.build(),
            ttsAvailable: BuiltValueNullFieldError.checkNotNull(
                ttsAvailable, r'MessageSummary', 'ttsAvailable'),
            turn: BuiltValueNullFieldError.checkNotNull(
                turn, r'MessageSummary', 'turn'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'toolCalls';
        _toolCalls?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'MessageSummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
