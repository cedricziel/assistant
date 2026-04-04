// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_conversation_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateConversationRequest extends CreateConversationRequest {
  @override
  final String? title;

  factory _$CreateConversationRequest(
          [void Function(CreateConversationRequestBuilder)? updates]) =>
      (CreateConversationRequestBuilder()..update(updates))._build();

  _$CreateConversationRequest._({this.title}) : super._();
  @override
  CreateConversationRequest rebuild(
          void Function(CreateConversationRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateConversationRequestBuilder toBuilder() =>
      CreateConversationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateConversationRequest && title == other.title;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateConversationRequest')
          ..add('title', title))
        .toString();
  }
}

class CreateConversationRequestBuilder
    implements
        Builder<CreateConversationRequest, CreateConversationRequestBuilder> {
  _$CreateConversationRequest? _$v;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  CreateConversationRequestBuilder() {
    CreateConversationRequest._defaults(this);
  }

  CreateConversationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _title = $v.title;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateConversationRequest other) {
    _$v = other as _$CreateConversationRequest;
  }

  @override
  void update(void Function(CreateConversationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateConversationRequest build() => _build();

  _$CreateConversationRequest _build() {
    final _$result = _$v ??
        _$CreateConversationRequest._(
          title: title,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
