// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'conversation_search_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConversationSearchSummary extends ConversationSearchSummary {
  @override
  final DateTime? archivedAt;
  @override
  final DateTime createdAt;
  @override
  final String id;
  @override
  final BuiltList<MessageSearchMatch> matches;
  @override
  final DateTime? pinnedAt;
  @override
  final String title;
  @override
  final BuiltList<SearchSegment> titleSegments;
  @override
  final DateTime updatedAt;

  factory _$ConversationSearchSummary(
          [void Function(ConversationSearchSummaryBuilder)? updates]) =>
      (ConversationSearchSummaryBuilder()..update(updates))._build();

  _$ConversationSearchSummary._(
      {this.archivedAt,
      required this.createdAt,
      required this.id,
      required this.matches,
      this.pinnedAt,
      required this.title,
      required this.titleSegments,
      required this.updatedAt})
      : super._();
  @override
  ConversationSearchSummary rebuild(
          void Function(ConversationSearchSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConversationSearchSummaryBuilder toBuilder() =>
      ConversationSearchSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConversationSearchSummary &&
        archivedAt == other.archivedAt &&
        createdAt == other.createdAt &&
        id == other.id &&
        matches == other.matches &&
        pinnedAt == other.pinnedAt &&
        title == other.title &&
        titleSegments == other.titleSegments &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, archivedAt.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, matches.hashCode);
    _$hash = $jc(_$hash, pinnedAt.hashCode);
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jc(_$hash, titleSegments.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConversationSearchSummary')
          ..add('archivedAt', archivedAt)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('matches', matches)
          ..add('pinnedAt', pinnedAt)
          ..add('title', title)
          ..add('titleSegments', titleSegments)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class ConversationSearchSummaryBuilder
    implements
        Builder<ConversationSearchSummary, ConversationSearchSummaryBuilder> {
  _$ConversationSearchSummary? _$v;

  DateTime? _archivedAt;
  DateTime? get archivedAt => _$this._archivedAt;
  set archivedAt(DateTime? archivedAt) => _$this._archivedAt = archivedAt;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  ListBuilder<MessageSearchMatch>? _matches;
  ListBuilder<MessageSearchMatch> get matches =>
      _$this._matches ??= ListBuilder<MessageSearchMatch>();
  set matches(ListBuilder<MessageSearchMatch>? matches) =>
      _$this._matches = matches;

  DateTime? _pinnedAt;
  DateTime? get pinnedAt => _$this._pinnedAt;
  set pinnedAt(DateTime? pinnedAt) => _$this._pinnedAt = pinnedAt;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  ListBuilder<SearchSegment>? _titleSegments;
  ListBuilder<SearchSegment> get titleSegments =>
      _$this._titleSegments ??= ListBuilder<SearchSegment>();
  set titleSegments(ListBuilder<SearchSegment>? titleSegments) =>
      _$this._titleSegments = titleSegments;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  ConversationSearchSummaryBuilder() {
    ConversationSearchSummary._defaults(this);
  }

  ConversationSearchSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _archivedAt = $v.archivedAt;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _matches = $v.matches.toBuilder();
      _pinnedAt = $v.pinnedAt;
      _title = $v.title;
      _titleSegments = $v.titleSegments.toBuilder();
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConversationSearchSummary other) {
    _$v = other as _$ConversationSearchSummary;
  }

  @override
  void update(void Function(ConversationSearchSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConversationSearchSummary build() => _build();

  _$ConversationSearchSummary _build() {
    _$ConversationSearchSummary _$result;
    try {
      _$result = _$v ??
          _$ConversationSearchSummary._(
            archivedAt: archivedAt,
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'ConversationSearchSummary', 'createdAt'),
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'ConversationSearchSummary', 'id'),
            matches: matches.build(),
            pinnedAt: pinnedAt,
            title: BuiltValueNullFieldError.checkNotNull(
                title, r'ConversationSearchSummary', 'title'),
            titleSegments: titleSegments.build(),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'ConversationSearchSummary', 'updatedAt'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'matches';
        matches.build();

        _$failedField = 'titleSegments';
        titleSegments.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ConversationSearchSummary', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
