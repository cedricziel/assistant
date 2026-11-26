// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'question_answer.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$QuestionAnswer extends QuestionAnswer {
  @override
  final String questionId;
  @override
  final BuiltList<String>? selectedOptionIds;
  @override
  final String? text;

  factory _$QuestionAnswer([void Function(QuestionAnswerBuilder)? updates]) =>
      (QuestionAnswerBuilder()..update(updates))._build();

  _$QuestionAnswer._(
      {required this.questionId, this.selectedOptionIds, this.text})
      : super._();
  @override
  QuestionAnswer rebuild(void Function(QuestionAnswerBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  QuestionAnswerBuilder toBuilder() => QuestionAnswerBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is QuestionAnswer &&
        questionId == other.questionId &&
        selectedOptionIds == other.selectedOptionIds &&
        text == other.text;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, questionId.hashCode);
    _$hash = $jc(_$hash, selectedOptionIds.hashCode);
    _$hash = $jc(_$hash, text.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'QuestionAnswer')
          ..add('questionId', questionId)
          ..add('selectedOptionIds', selectedOptionIds)
          ..add('text', text))
        .toString();
  }
}

class QuestionAnswerBuilder
    implements Builder<QuestionAnswer, QuestionAnswerBuilder> {
  _$QuestionAnswer? _$v;

  String? _questionId;
  String? get questionId => _$this._questionId;
  set questionId(String? questionId) => _$this._questionId = questionId;

  ListBuilder<String>? _selectedOptionIds;
  ListBuilder<String> get selectedOptionIds =>
      _$this._selectedOptionIds ??= ListBuilder<String>();
  set selectedOptionIds(ListBuilder<String>? selectedOptionIds) =>
      _$this._selectedOptionIds = selectedOptionIds;

  String? _text;
  String? get text => _$this._text;
  set text(String? text) => _$this._text = text;

  QuestionAnswerBuilder() {
    QuestionAnswer._defaults(this);
  }

  QuestionAnswerBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _questionId = $v.questionId;
      _selectedOptionIds = $v.selectedOptionIds?.toBuilder();
      _text = $v.text;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(QuestionAnswer other) {
    _$v = other as _$QuestionAnswer;
  }

  @override
  void update(void Function(QuestionAnswerBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  QuestionAnswer build() => _build();

  _$QuestionAnswer _build() {
    _$QuestionAnswer _$result;
    try {
      _$result = _$v ??
          _$QuestionAnswer._(
            questionId: BuiltValueNullFieldError.checkNotNull(
                questionId, r'QuestionAnswer', 'questionId'),
            selectedOptionIds: _selectedOptionIds?.build(),
            text: text,
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'selectedOptionIds';
        _selectedOptionIds?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'QuestionAnswer', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
