// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_conversation_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateConversationRequest extends UpdateConversationRequest {
  @override
  final bool? pinned;
  @override
  final String? title;

  factory _$UpdateConversationRequest(
          [void Function(UpdateConversationRequestBuilder)? updates]) =>
      (UpdateConversationRequestBuilder()..update(updates))._build();

  _$UpdateConversationRequest._({this.pinned, this.title}) : super._();
  @override
  UpdateConversationRequest rebuild(
          void Function(UpdateConversationRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdateConversationRequestBuilder toBuilder() =>
      UpdateConversationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateConversationRequest &&
        pinned == other.pinned &&
        title == other.title;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, pinned.hashCode);
    _$hash = $jc(_$hash, title.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateConversationRequest')
          ..add('pinned', pinned)
          ..add('title', title))
        .toString();
  }
}

class UpdateConversationRequestBuilder
    implements
        Builder<UpdateConversationRequest, UpdateConversationRequestBuilder> {
  _$UpdateConversationRequest? _$v;

  bool? _pinned;
  bool? get pinned => _$this._pinned;
  set pinned(bool? pinned) => _$this._pinned = pinned;

  String? _title;
  String? get title => _$this._title;
  set title(String? title) => _$this._title = title;

  UpdateConversationRequestBuilder() {
    UpdateConversationRequest._defaults(this);
  }

  UpdateConversationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _pinned = $v.pinned;
      _title = $v.title;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateConversationRequest other) {
    _$v = other as _$UpdateConversationRequest;
  }

  @override
  void update(void Function(UpdateConversationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateConversationRequest build() => _build();

  _$UpdateConversationRequest _build() {
    final _$result = _$v ??
        _$UpdateConversationRequest._(
          pinned: pinned,
          title: title,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
