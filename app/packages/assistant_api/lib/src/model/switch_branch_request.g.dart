// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'switch_branch_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SwitchBranchRequest extends SwitchBranchRequest {
  @override
  final String messageId;

  factory _$SwitchBranchRequest(
          [void Function(SwitchBranchRequestBuilder)? updates]) =>
      (SwitchBranchRequestBuilder()..update(updates))._build();

  _$SwitchBranchRequest._({required this.messageId}) : super._();
  @override
  SwitchBranchRequest rebuild(
          void Function(SwitchBranchRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SwitchBranchRequestBuilder toBuilder() =>
      SwitchBranchRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SwitchBranchRequest && messageId == other.messageId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, messageId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SwitchBranchRequest')
          ..add('messageId', messageId))
        .toString();
  }
}

class SwitchBranchRequestBuilder
    implements Builder<SwitchBranchRequest, SwitchBranchRequestBuilder> {
  _$SwitchBranchRequest? _$v;

  String? _messageId;
  String? get messageId => _$this._messageId;
  set messageId(String? messageId) => _$this._messageId = messageId;

  SwitchBranchRequestBuilder() {
    SwitchBranchRequest._defaults(this);
  }

  SwitchBranchRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _messageId = $v.messageId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SwitchBranchRequest other) {
    _$v = other as _$SwitchBranchRequest;
  }

  @override
  void update(void Function(SwitchBranchRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SwitchBranchRequest build() => _build();

  _$SwitchBranchRequest _build() {
    final _$result = _$v ??
        _$SwitchBranchRequest._(
          messageId: BuiltValueNullFieldError.checkNotNull(
              messageId, r'SwitchBranchRequest', 'messageId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
