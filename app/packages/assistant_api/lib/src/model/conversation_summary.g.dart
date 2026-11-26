// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'conversation_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConversationSummary extends ConversationSummary {
  @override
  final DateTime? archivedAt;
  @override
  final DateTime createdAt;
  @override
  final String id;
  @override
  final DateTime? pinnedAt;
  @override
  final String title;
  @override
  final DateTime updatedAt;

  factory _$ConversationSummary(
          [void Function(ConversationSummaryBuilder)? updates]) =>
      (ConversationSummaryBuilder()..update(updates))._build();

  _$ConversationSummary._(
      {this.archivedAt,
      required this.createdAt,
      required this.id,
      this.pinnedAt,
      required this.title,
      required this.updatedAt})
      : super._();
  @override
  ConversationSummary rebuild(
          void Function(ConversationSummaryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConversationSummaryBuilder toBuilder() =>
      ConversationSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConversationSummary &&
        archivedAt == other.archivedAt &&
        createdAt == other.createdAt &&
        id == other.id &&
        pinnedAt == other.pinnedAt &&
        title == other.title &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, archivedAt.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, pinnedAt.hashCode);
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConversationSummary')
          ..add('archivedAt', archivedAt)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('pinnedAt', pinnedAt)
          ..add('title', title)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class ConversationSummaryBuilder
    implements Builder<ConversationSummary, ConversationSummaryBuilder> {
  _$ConversationSummary? _$v;

  DateTime? _archivedAt;
  DateTime? get archivedAt => _$this._archivedAt;
  set archivedAt(DateTime? archivedAt) => _$this._archivedAt = archivedAt;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  DateTime? _pinnedAt;
  DateTime? get pinnedAt => _$this._pinnedAt;
  set pinnedAt(DateTime? pinnedAt) => _$this._pinnedAt = pinnedAt;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  ConversationSummaryBuilder() {
    ConversationSummary._defaults(this);
  }

  ConversationSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _archivedAt = $v.archivedAt;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _pinnedAt = $v.pinnedAt;
      _title = $v.title;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConversationSummary other) {
    _$v = other as _$ConversationSummary;
  }

  @override
  void update(void Function(ConversationSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConversationSummary build() => _build();

  _$ConversationSummary _build() {
    final _$result = _$v ??
        _$ConversationSummary._(
          archivedAt: archivedAt,
          createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt, r'ConversationSummary', 'createdAt'),
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'ConversationSummary', 'id'),
          pinnedAt: pinnedAt,
          title: BuiltValueNullFieldError.checkNotNull(
              title, r'ConversationSummary', 'title'),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt, r'ConversationSummary', 'updatedAt'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
