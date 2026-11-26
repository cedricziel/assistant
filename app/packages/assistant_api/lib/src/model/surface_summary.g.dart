// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'surface_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SurfaceSummary extends SurfaceSummary {
  @override
  final String? actionMessageId;
  @override
  final int anchorCallIndex;
  @override
  final String anchorMessageId;
  @override
  final bool answerable;
  @override
  final DateTime createdAt;
  @override
  final String fallbackText;
  @override
  final BuiltList<JsonObject> messages;
  @override
  final String status;
  @override
  final JsonObject? submittedAction;
  @override
  final String surfaceId;
  @override
  final String title;
  @override
  final String? toolCallId;
  @override
  final int turn;

  factory _$SurfaceSummary([void Function(SurfaceSummaryBuilder)? updates]) =>
      (SurfaceSummaryBuilder()..update(updates))._build();

  _$SurfaceSummary._(
      {this.actionMessageId,
      required this.anchorCallIndex,
      required this.anchorMessageId,
      required this.answerable,
      required this.createdAt,
      required this.fallbackText,
      required this.messages,
      required this.status,
      this.submittedAction,
      required this.surfaceId,
      required this.title,
      this.toolCallId,
      required this.turn})
      : super._();
  @override
  SurfaceSummary rebuild(void Function(SurfaceSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SurfaceSummaryBuilder toBuilder() => SurfaceSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SurfaceSummary &&
        actionMessageId == other.actionMessageId &&
        anchorCallIndex == other.anchorCallIndex &&
        anchorMessageId == other.anchorMessageId &&
        answerable == other.answerable &&
        createdAt == other.createdAt &&
        fallbackText == other.fallbackText &&
        messages == other.messages &&
        status == other.status &&
        submittedAction == other.submittedAction &&
        surfaceId == other.surfaceId &&
        title == other.title &&
        toolCallId == other.toolCallId &&
        turn == other.turn;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, actionMessageId.hashCode);
    _$hash = $jc(_$hash, anchorCallIndex.hashCode);
    _$hash = $jc(_$hash, anchorMessageId.hashCode);
    _$hash = $jc(_$hash, answerable.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, fallbackText.hashCode);
    _$hash = $jc(_$hash, messages.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, submittedAction.hashCode);
    _$hash = $jc(_$hash, surfaceId.hashCode);
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jc(_$hash, toolCallId.hashCode);
    _$hash = $jc(_$hash, turn.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SurfaceSummary')
          ..add('actionMessageId', actionMessageId)
          ..add('anchorCallIndex', anchorCallIndex)
          ..add('anchorMessageId', anchorMessageId)
          ..add('answerable', answerable)
          ..add('createdAt', createdAt)
          ..add('fallbackText', fallbackText)
          ..add('messages', messages)
          ..add('status', status)
          ..add('submittedAction', submittedAction)
          ..add('surfaceId', surfaceId)
          ..add('title', title)
          ..add('toolCallId', toolCallId)
          ..add('turn', turn))
        .toString();
  }
}

class SurfaceSummaryBuilder
    implements Builder<SurfaceSummary, SurfaceSummaryBuilder> {
  _$SurfaceSummary? _$v;

  String? _actionMessageId;
  String? get actionMessageId => _$this._actionMessageId;
  set actionMessageId(String? actionMessageId) =>
      _$this._actionMessageId = actionMessageId;

  int? _anchorCallIndex;
  int? get anchorCallIndex => _$this._anchorCallIndex;
  set anchorCallIndex(int? anchorCallIndex) =>
      _$this._anchorCallIndex = anchorCallIndex;

  String? _anchorMessageId;
  String? get anchorMessageId => _$this._anchorMessageId;
  set anchorMessageId(String? anchorMessageId) =>
      _$this._anchorMessageId = anchorMessageId;

  bool? _answerable;
  bool? get answerable => _$this._answerable;
  set answerable(bool? answerable) => _$this._answerable = answerable;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _fallbackText;
  String? get fallbackText => _$this._fallbackText;
  set fallbackText(String? fallbackText) => _$this._fallbackText = fallbackText;

  ListBuilder<JsonObject>? _messages;
  ListBuilder<JsonObject> get messages =>
      _$this._messages ??= ListBuilder<JsonObject>();
  set messages(ListBuilder<JsonObject>? messages) =>
      _$this._messages = messages;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  JsonObject? _submittedAction;
  JsonObject? get submittedAction => _$this._submittedAction;
  set submittedAction(JsonObject? submittedAction) =>
      _$this._submittedAction = submittedAction;

  String? _surfaceId;
  String? get surfaceId => _$this._surfaceId;
  set surfaceId(String? surfaceId) => _$this._surfaceId = surfaceId;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  String? _toolCallId;
  String? get toolCallId => _$this._toolCallId;
  set toolCallId(String? toolCallId) => _$this._toolCallId = toolCallId;

  int? _turn;
  int? get turn => _$this._turn;
  set turn(int? turn) => _$this._turn = turn;

  SurfaceSummaryBuilder() {
    SurfaceSummary._defaults(this);
  }

  SurfaceSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _actionMessageId = $v.actionMessageId;
      _anchorCallIndex = $v.anchorCallIndex;
      _anchorMessageId = $v.anchorMessageId;
      _answerable = $v.answerable;
      _createdAt = $v.createdAt;
      _fallbackText = $v.fallbackText;
      _messages = $v.messages.toBuilder();
      _status = $v.status;
      _submittedAction = $v.submittedAction;
      _surfaceId = $v.surfaceId;
      _title = $v.title;
      _toolCallId = $v.toolCallId;
      _turn = $v.turn;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SurfaceSummary other) {
    _$v = other as _$SurfaceSummary;
  }

  @override
  void update(void Function(SurfaceSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SurfaceSummary build() => _build();

  _$SurfaceSummary _build() {
    _$SurfaceSummary _$result;
    try {
      _$result = _$v ??
          _$SurfaceSummary._(
            actionMessageId: actionMessageId,
            anchorCallIndex: BuiltValueNullFieldError.checkNotNull(
                anchorCallIndex, r'SurfaceSummary', 'anchorCallIndex'),
            anchorMessageId: BuiltValueNullFieldError.checkNotNull(
                anchorMessageId, r'SurfaceSummary', 'anchorMessageId'),
            answerable: BuiltValueNullFieldError.checkNotNull(
                answerable, r'SurfaceSummary', 'answerable'),
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'SurfaceSummary', 'createdAt'),
            fallbackText: BuiltValueNullFieldError.checkNotNull(
                fallbackText, r'SurfaceSummary', 'fallbackText'),
            messages: messages.build(),
            status: BuiltValueNullFieldError.checkNotNull(
                status, r'SurfaceSummary', 'status'),
            submittedAction: submittedAction,
            surfaceId: BuiltValueNullFieldError.checkNotNull(
                surfaceId, r'SurfaceSummary', 'surfaceId'),
            title: BuiltValueNullFieldError.checkNotNull(
                title, r'SurfaceSummary', 'title'),
            toolCallId: toolCallId,
            turn: BuiltValueNullFieldError.checkNotNull(
                turn, r'SurfaceSummary', 'turn'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'messages';
        messages.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'SurfaceSummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
