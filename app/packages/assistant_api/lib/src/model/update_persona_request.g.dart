// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_persona_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdatePersonaRequest extends UpdatePersonaRequest {
  @override
  final BuiltList<String>? starterPrompts;

  factory _$UpdatePersonaRequest(
          [void Function(UpdatePersonaRequestBuilder)? updates]) =>
      (UpdatePersonaRequestBuilder()..update(updates))._build();

  _$UpdatePersonaRequest._({this.starterPrompts}) : super._();
  @override
  UpdatePersonaRequest rebuild(
          void Function(UpdatePersonaRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UpdatePersonaRequestBuilder toBuilder() =>
      UpdatePersonaRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdatePersonaRequest &&
        starterPrompts == other.starterPrompts;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, starterPrompts.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdatePersonaRequest')
          ..add('starterPrompts', starterPrompts))
        .toString();
  }
}

class UpdatePersonaRequestBuilder
    implements Builder<UpdatePersonaRequest, UpdatePersonaRequestBuilder> {
  _$UpdatePersonaRequest? _$v;

  ListBuilder<String>? _starterPrompts;
  ListBuilder<String> get starterPrompts =>
      _$this._starterPrompts ??= ListBuilder<String>();
  set starterPrompts(ListBuilder<String>? starterPrompts) =>
      _$this._starterPrompts = starterPrompts;

  UpdatePersonaRequestBuilder() {
    UpdatePersonaRequest._defaults(this);
  }

  UpdatePersonaRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _starterPrompts = $v.starterPrompts?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdatePersonaRequest other) {
    _$v = other as _$UpdatePersonaRequest;
  }

  @override
  void update(void Function(UpdatePersonaRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdatePersonaRequest build() => _build();

  _$UpdatePersonaRequest _build() {
    _$UpdatePersonaRequest _$result;
    try {
      _$result = _$v ??
          _$UpdatePersonaRequest._(
            starterPrompts: _starterPrompts?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'starterPrompts';
        _starterPrompts?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'UpdatePersonaRequest', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
