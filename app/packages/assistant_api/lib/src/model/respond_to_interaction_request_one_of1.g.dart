// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'respond_to_interaction_request_one_of1.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const RespondToInteractionRequestOneOf1KindEnum
    _$respondToInteractionRequestOneOf1KindEnum_question =
    const RespondToInteractionRequestOneOf1KindEnum._('question');
const RespondToInteractionRequestOneOf1KindEnum
    _$respondToInteractionRequestOneOf1KindEnum_unknownDefaultOpenApi =
    const RespondToInteractionRequestOneOf1KindEnum._('unknownDefaultOpenApi');

RespondToInteractionRequestOneOf1KindEnum
    _$respondToInteractionRequestOneOf1KindEnumValueOf(String name) {
  switch (name) {
    case 'question':
      return _$respondToInteractionRequestOneOf1KindEnum_question;
    case 'unknownDefaultOpenApi':
      return _$respondToInteractionRequestOneOf1KindEnum_unknownDefaultOpenApi;
    default:
      return _$respondToInteractionRequestOneOf1KindEnum_unknownDefaultOpenApi;
  }
}

final BuiltSet<RespondToInteractionRequestOneOf1KindEnum>
    _$respondToInteractionRequestOneOf1KindEnumValues = BuiltSet<
        RespondToInteractionRequestOneOf1KindEnum>(const <RespondToInteractionRequestOneOf1KindEnum>[
  _$respondToInteractionRequestOneOf1KindEnum_question,
  _$respondToInteractionRequestOneOf1KindEnum_unknownDefaultOpenApi,
]);

Serializer<RespondToInteractionRequestOneOf1KindEnum>
    _$respondToInteractionRequestOneOf1KindEnumSerializer =
    _$RespondToInteractionRequestOneOf1KindEnumSerializer();

class _$RespondToInteractionRequestOneOf1KindEnumSerializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf1KindEnum> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'question': 'question',
    'unknownDefaultOpenApi': 'unknown_default_open_api',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'question': 'question',
    'unknown_default_open_api': 'unknownDefaultOpenApi',
  };

  @override
  final Iterable<Type> types = const <Type>[
    RespondToInteractionRequestOneOf1KindEnum
  ];
  @override
  final String wireName = 'RespondToInteractionRequestOneOf1KindEnum';

  @override
  Object serialize(Serializers serializers,
          RespondToInteractionRequestOneOf1KindEnum object,
          {FullType specifiedType = FullType.unspecified}) =>
      _toWire[object.name] ?? object.name;

  @override
  RespondToInteractionRequestOneOf1KindEnum deserialize(
          Serializers serializers, Object serialized,
          {FullType specifiedType = FullType.unspecified}) =>
      RespondToInteractionRequestOneOf1KindEnum.valueOf(
          _fromWire[serialized] ?? (serialized is String ? serialized : ''));
}

class _$RespondToInteractionRequestOneOf1
    extends RespondToInteractionRequestOneOf1 {
  @override
  final BuiltList<QuestionAnswer> answers;
  @override
  final RespondToInteractionRequestOneOf1KindEnum kind;

  factory _$RespondToInteractionRequestOneOf1(
          [void Function(RespondToInteractionRequestOneOf1Builder)? updates]) =>
      (RespondToInteractionRequestOneOf1Builder()..update(updates))._build();

  _$RespondToInteractionRequestOneOf1._(
      {required this.answers, required this.kind})
      : super._();
  @override
  RespondToInteractionRequestOneOf1 rebuild(
          void Function(RespondToInteractionRequestOneOf1Builder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RespondToInteractionRequestOneOf1Builder toBuilder() =>
      RespondToInteractionRequestOneOf1Builder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RespondToInteractionRequestOneOf1 &&
        answers == other.answers &&
        kind == other.kind;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, answers.hashCode);
    _$hash = $jc(_$hash, kind.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RespondToInteractionRequestOneOf1')
          ..add('answers', answers)
          ..add('kind', kind))
        .toString();
  }
}

class RespondToInteractionRequestOneOf1Builder
    implements
        Builder<RespondToInteractionRequestOneOf1,
            RespondToInteractionRequestOneOf1Builder> {
  _$RespondToInteractionRequestOneOf1? _$v;

  ListBuilder<QuestionAnswer>? _answers;
  ListBuilder<QuestionAnswer> get answers =>
      _$this._answers ??= ListBuilder<QuestionAnswer>();
  set answers(ListBuilder<QuestionAnswer>? answers) =>
      _$this._answers = answers;

  RespondToInteractionRequestOneOf1KindEnum? _kind;
  RespondToInteractionRequestOneOf1KindEnum? get kind => _$this._kind;
  set kind(RespondToInteractionRequestOneOf1KindEnum? kind) =>
      _$this._kind = kind;

  RespondToInteractionRequestOneOf1Builder() {
    RespondToInteractionRequestOneOf1._defaults(this);
  }

  RespondToInteractionRequestOneOf1Builder get _$this {
    final $v = _$v;
    if ($v != null) {
      _answers = $v.answers.toBuilder();
      _kind = $v.kind;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RespondToInteractionRequestOneOf1 other) {
    _$v = other as _$RespondToInteractionRequestOneOf1;
  }

  @override
  void update(
      void Function(RespondToInteractionRequestOneOf1Builder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RespondToInteractionRequestOneOf1 build() => _build();

  _$RespondToInteractionRequestOneOf1 _build() {
    _$RespondToInteractionRequestOneOf1 _$result;
    try {
      _$result = _$v ??
          _$RespondToInteractionRequestOneOf1._(
            answers: answers.build(),
            kind: BuiltValueNullFieldError.checkNotNull(
                kind, r'RespondToInteractionRequestOneOf1', 'kind'),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'answers';
        answers.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'RespondToInteractionRequestOneOf1', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
