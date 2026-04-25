// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'complete_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CompleteResponse extends CompleteResponse {
  @override
  final String code;
  @override
  final String? state;

  factory _$CompleteResponse(
          [void Function(CompleteResponseBuilder)? updates]) =>
      (CompleteResponseBuilder()..update(updates))._build();

  _$CompleteResponse._({required this.code, this.state}) : super._();
  @override
  CompleteResponse rebuild(void Function(CompleteResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CompleteResponseBuilder toBuilder() =>
      CompleteResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CompleteResponse &&
        code == other.code &&
        state == other.state;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, code.hashCode);
    _$hash = $jc(_$hash, state.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CompleteResponse')
          ..add('code', code)
          ..add('state', state))
        .toString();
  }
}

class CompleteResponseBuilder
    implements Builder<CompleteResponse, CompleteResponseBuilder> {
  _$CompleteResponse? _$v;

  String? _code;
  String? get code => _$this._code;
  set code(String? code) => _$this._code = code;

  String? _state;
  String? get state => _$this._state;
  set state(String? state) => _$this._state = state;

  CompleteResponseBuilder() {
    CompleteResponse._defaults(this);
  }

  CompleteResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _code = $v.code;
      _state = $v.state;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CompleteResponse other) {
    _$v = other as _$CompleteResponse;
  }

  @override
  void update(void Function(CompleteResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CompleteResponse build() => _build();

  _$CompleteResponse _build() {
    final _$result = _$v ??
        _$CompleteResponse._(
          code: BuiltValueNullFieldError.checkNotNull(
              code, r'CompleteResponse', 'code'),
          state: state,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
