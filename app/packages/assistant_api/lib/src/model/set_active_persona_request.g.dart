// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_active_persona_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetActivePersonaRequest extends SetActivePersonaRequest {
  @override
  final String id;

  factory _$SetActivePersonaRequest(
          [void Function(SetActivePersonaRequestBuilder)? updates]) =>
      (SetActivePersonaRequestBuilder()..update(updates))._build();

  _$SetActivePersonaRequest._({required this.id}) : super._();
  @override
  SetActivePersonaRequest rebuild(
          void Function(SetActivePersonaRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetActivePersonaRequestBuilder toBuilder() =>
      SetActivePersonaRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetActivePersonaRequest && id == other.id;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetActivePersonaRequest')
          ..add('id', id))
        .toString();
  }
}

class SetActivePersonaRequestBuilder
    implements
        Builder<SetActivePersonaRequest, SetActivePersonaRequestBuilder> {
  _$SetActivePersonaRequest? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  SetActivePersonaRequestBuilder() {
    SetActivePersonaRequest._defaults(this);
  }

  SetActivePersonaRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetActivePersonaRequest other) {
    _$v = other as _$SetActivePersonaRequest;
  }

  @override
  void update(void Function(SetActivePersonaRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetActivePersonaRequest build() => _build();

  _$SetActivePersonaRequest _build() {
    final _$result = _$v ??
        _$SetActivePersonaRequest._(
          id: BuiltValueNullFieldError.checkNotNull(
              id, r'SetActivePersonaRequest', 'id'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
