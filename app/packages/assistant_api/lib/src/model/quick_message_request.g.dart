// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'quick_message_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$QuickMessageRequest extends QuickMessageRequest {
  @override
  final String? context;
  @override
  final String message;
  @override
  final String? personaId;

  factory _$QuickMessageRequest(
          [void Function(QuickMessageRequestBuilder)? updates]) =>
      (QuickMessageRequestBuilder()..update(updates))._build();

  _$QuickMessageRequest._({this.context, required this.message, this.personaId})
      : super._();
  @override
  QuickMessageRequest rebuild(
          void Function(QuickMessageRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  QuickMessageRequestBuilder toBuilder() =>
      QuickMessageRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is QuickMessageRequest &&
        context == other.context &&
        message == other.message &&
        personaId == other.personaId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, context.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'QuickMessageRequest')
          ..add('context', context)
          ..add('message', message)
          ..add('personaId', personaId))
        .toString();
  }
}

class QuickMessageRequestBuilder
    implements Builder<QuickMessageRequest, QuickMessageRequestBuilder> {
  _$QuickMessageRequest? _$v;

  String? _context;
  String? get context => _$this._context;
  set context(String? context) => _$this._context = context;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  QuickMessageRequestBuilder() {
    QuickMessageRequest._defaults(this);
  }

  QuickMessageRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _context = $v.context;
      _message = $v.message;
      _personaId = $v.personaId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(QuickMessageRequest other) {
    _$v = other as _$QuickMessageRequest;
  }

  @override
  void update(void Function(QuickMessageRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  QuickMessageRequest build() => _build();

  _$QuickMessageRequest _build() {
    final _$result = _$v ??
        _$QuickMessageRequest._(
          context: context,
          message: BuiltValueNullFieldError.checkNotNull(
              message, r'QuickMessageRequest', 'message'),
          personaId: personaId,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
