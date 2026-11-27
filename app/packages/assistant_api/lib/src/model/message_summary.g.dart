// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'message_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$MessageSummary extends MessageSummary {
  @override
  final BuiltList<AttachmentMetaResponse>? attachments;
  @override
  final BranchInfo? branch;
  @override
  final String content;
  @override
  final DateTime createdAt;
  @override
  final MessageFeedbackSummary? feedback;
  @override
  final String? finishReason;
  @override
  final String id;
  @override
  final String? parentId;
  @override
  final BranchInfo? replyBranch;
  @override
  final String role;
  @override
  final String? skillName;
  @override
  final MessageTimingSummary? timing;
  @override
  final BuiltList<ToolCallSummary>? toolCalls;
  @override
  final bool ttsAvailable;
  @override
  final int turn;

  factory _$MessageSummary([void Function(MessageSummaryBuilder)? updates]) =>
      (MessageSummaryBuilder()..update(updates))._build();

  _$MessageSummary._(
      {this.attachments,
      this.branch,
      required this.content,
      required this.createdAt,
      this.feedback,
      this.finishReason,
      required this.id,
      this.parentId,
      this.replyBranch,
      required this.role,
      this.skillName,
      this.timing,
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
        attachments == other.attachments &&
        branch == other.branch &&
        content == other.content &&
        createdAt == other.createdAt &&
        feedback == other.feedback &&
        finishReason == other.finishReason &&
        id == other.id &&
        parentId == other.parentId &&
        replyBranch == other.replyBranch &&
        role == other.role &&
        skillName == other.skillName &&
        timing == other.timing &&
        toolCalls == other.toolCalls &&
        ttsAvailable == other.ttsAvailable &&
        turn == other.turn;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, attachments.hashCode);
    _$hash = $jc(_$hash, branch.hashCode);
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, feedback.hashCode);
    _$hash = $jc(_$hash, finishReason.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, parentId.hashCode);
    _$hash = $jc(_$hash, replyBranch.hashCode);
    _$hash = $jc(_$hash, role.hashCode);
    _$hash = $jc(_$hash, skillName.hashCode);
    _$hash = $jc(_$hash, timing.hashCode);
    _$hash = $jc(_$hash, toolCalls.hashCode);
    _$hash = $jc(_$hash, ttsAvailable.hashCode);
    _$hash = $jc(_$hash, turn.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'MessageSummary')
          ..add('attachments', attachments)
          ..add('branch', branch)
          ..add('content', content)
          ..add('createdAt', createdAt)
          ..add('feedback', feedback)
          ..add('finishReason', finishReason)
          ..add('id', id)
          ..add('parentId', parentId)
          ..add('replyBranch', replyBranch)
          ..add('role', role)
          ..add('skillName', skillName)
          ..add('timing', timing)
          ..add('toolCalls', toolCalls)
          ..add('ttsAvailable', ttsAvailable)
          ..add('turn', turn))
        .toString();
  }
}

class MessageSummaryBuilder
    implements Builder<MessageSummary, MessageSummaryBuilder> {
  _$MessageSummary? _$v;

  ListBuilder<AttachmentMetaResponse>? _attachments;
  ListBuilder<AttachmentMetaResponse> get attachments =>
      _$this._attachments ??= ListBuilder<AttachmentMetaResponse>();
  set attachments(ListBuilder<AttachmentMetaResponse>? attachments) =>
      _$this._attachments = attachments;

  BranchInfoBuilder? _branch;
  BranchInfoBuilder get branch => _$this._branch ??= BranchInfoBuilder();
  set branch(BranchInfoBuilder? branch) => _$this._branch = branch;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  MessageFeedbackSummaryBuilder? _feedback;
  MessageFeedbackSummaryBuilder get feedback =>
      _$this._feedback ??= MessageFeedbackSummaryBuilder();
  set feedback(MessageFeedbackSummaryBuilder? feedback) =>
      _$this._feedback = feedback;

  String? _finishReason;
  String? get finishReason => _$this._finishReason;
  set finishReason(String? finishReason) => _$this._finishReason = finishReason;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _parentId;
  String? get parentId => _$this._parentId;
  set parentId(String? parentId) => _$this._parentId = parentId;

  BranchInfoBuilder? _replyBranch;
  BranchInfoBuilder get replyBranch =>
      _$this._replyBranch ??= BranchInfoBuilder();
  set replyBranch(BranchInfoBuilder? replyBranch) =>
      _$this._replyBranch = replyBranch;

  String? _role;
  String? get role => _$this._role;
  set role(String? role) => _$this._role = role;

  String? _skillName;
  String? get skillName => _$this._skillName;
  set skillName(String? skillName) => _$this._skillName = skillName;

  MessageTimingSummaryBuilder? _timing;
  MessageTimingSummaryBuilder get timing =>
      _$this._timing ??= MessageTimingSummaryBuilder();
  set timing(MessageTimingSummaryBuilder? timing) => _$this._timing = timing;

  ListBuilder<ToolCallSummary>? _toolCalls;
  ListBuilder<ToolCallSummary> get toolCalls =>
      _$this._toolCalls ??= ListBuilder<ToolCallSummary>();
  set toolCalls(ListBuilder<ToolCallSummary>? toolCalls) =>
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
      _attachments = $v.attachments?.toBuilder();
      _branch = $v.branch?.toBuilder();
      _content = $v.content;
      _createdAt = $v.createdAt;
      _feedback = $v.feedback?.toBuilder();
      _finishReason = $v.finishReason;
      _id = $v.id;
      _parentId = $v.parentId;
      _replyBranch = $v.replyBranch?.toBuilder();
      _role = $v.role;
      _skillName = $v.skillName;
      _timing = $v.timing?.toBuilder();
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
            attachments: _attachments?.build(),
            branch: _branch?.build(),
            content: BuiltValueNullFieldError.checkNotNull(
                content, r'MessageSummary', 'content'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'MessageSummary', 'createdAt'),
            feedback: _feedback?.build(),
            finishReason: finishReason,
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'MessageSummary', 'id'),
            parentId: parentId,
            replyBranch: _replyBranch?.build(),
            role: BuiltValueNullFieldError.checkNotNull(
                role, r'MessageSummary', 'role'),
            skillName: skillName,
            timing: _timing?.build(),
            toolCalls: _toolCalls?.build(),
            ttsAvailable: BuiltValueNullFieldError.checkNotNull(
                ttsAvailable, r'MessageSummary', 'ttsAvailable'),
            turn: BuiltValueNullFieldError.checkNotNull(
                turn, r'MessageSummary', 'turn'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'attachments';
        _attachments?.build();
        _$failedField = 'branch';
        _branch?.build();

        _$failedField = 'feedback';
        _feedback?.build();

        _$failedField = 'replyBranch';
        _replyBranch?.build();

        _$failedField = 'timing';
        _timing?.build();
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
