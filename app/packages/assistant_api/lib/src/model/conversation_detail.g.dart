// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'conversation_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConversationDetail extends ConversationDetail {
  @override
  final DateTime createdAt;
  @override
  final String id;
  @override
  final BuiltList<MessageSummary> messages;
  @override
  final String title;
  @override
  final DateTime updatedAt;

  factory _$ConversationDetail(
          [void Function(ConversationDetailBuilder)? updates]) =>
      (ConversationDetailBuilder()..update(updates))._build();

  _$ConversationDetail._(
      {required this.createdAt,
      required this.id,
      required this.messages,
      required this.title,
      required this.updatedAt})
      : super._();
  @override
  ConversationDetail rebuild(
          void Function(ConversationDetailBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConversationDetailBuilder toBuilder() =>
      ConversationDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConversationDetail &&
        createdAt == other.createdAt &&
        id == other.id &&
        messages == other.messages &&
        title == other.title &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, messages.hashCode);
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ConversationDetail')
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('messages', messages)
          ..add('title', title)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class ConversationDetailBuilder
    implements Builder<ConversationDetail, ConversationDetailBuilder> {
  _$ConversationDetail? _$v;

  DateTime? _createdAt;
  DateTime? get createdAt => _$this._createdAt;
  set createdAt(DateTime? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  ListBuilder<MessageSummary>? _messages;
  ListBuilder<MessageSummary> get messages =>
      _$this._messages ??= ListBuilder<MessageSummary>();
  set messages(ListBuilder<MessageSummary>? messages) =>
      _$this._messages = messages;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  DateTime? _updatedAt;
  DateTime? get updatedAt => _$this._updatedAt;
  set updatedAt(DateTime? updatedAt) => _$this._updatedAt = updatedAt;

  ConversationDetailBuilder() {
    ConversationDetail._defaults(this);
  }

  ConversationDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _id = $v.id;
      _messages = $v.messages.toBuilder();
      _title = $v.title;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConversationDetail other) {
    _$v = other as _$ConversationDetail;
  }

  @override
  void update(void Function(ConversationDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConversationDetail build() => _build();

  _$ConversationDetail _build() {
    _$ConversationDetail _$result;
    try {
      _$result = _$v ??
          _$ConversationDetail._(
            createdAt: BuiltValueNullFieldError.checkNotNull(
                createdAt, r'ConversationDetail', 'createdAt'),
            id: BuiltValueNullFieldError.checkNotNull(
                id, r'ConversationDetail', 'id'),
            messages: messages.build(),
            title: BuiltValueNullFieldError.checkNotNull(
                title, r'ConversationDetail', 'title'),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
                updatedAt, r'ConversationDetail', 'updatedAt'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'messages';
        messages.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'ConversationDetail', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
