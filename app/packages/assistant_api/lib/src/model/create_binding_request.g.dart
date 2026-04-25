// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_binding_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateBindingRequest extends CreateBindingRequest {
  @override
  final String interfaceInstanceId;
  @override
  final String personaId;

  factory _$CreateBindingRequest(
          [void Function(CreateBindingRequestBuilder)? updates]) =>
      (CreateBindingRequestBuilder()..update(updates))._build();

  _$CreateBindingRequest._(
      {required this.interfaceInstanceId, required this.personaId})
      : super._();
  @override
  CreateBindingRequest rebuild(
          void Function(CreateBindingRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateBindingRequestBuilder toBuilder() =>
      CreateBindingRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateBindingRequest &&
        interfaceInstanceId == other.interfaceInstanceId &&
        personaId == other.personaId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, interfaceInstanceId.hashCode);
    _$hash = $jc(_$hash, personaId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateBindingRequest')
          ..add('interfaceInstanceId', interfaceInstanceId)
          ..add('personaId', personaId))
        .toString();
  }
}

class CreateBindingRequestBuilder
    implements Builder<CreateBindingRequest, CreateBindingRequestBuilder> {
  _$CreateBindingRequest? _$v;

  String? _interfaceInstanceId;
  String? get interfaceInstanceId => _$this._interfaceInstanceId;
  set interfaceInstanceId(String? interfaceInstanceId) =>
      _$this._interfaceInstanceId = interfaceInstanceId;

  String? _personaId;
  String? get personaId => _$this._personaId;
  set personaId(String? personaId) => _$this._personaId = personaId;

  CreateBindingRequestBuilder() {
    CreateBindingRequest._defaults(this);
  }

  CreateBindingRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _interfaceInstanceId = $v.interfaceInstanceId;
      _personaId = $v.personaId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateBindingRequest other) {
    _$v = other as _$CreateBindingRequest;
  }

  @override
  void update(void Function(CreateBindingRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateBindingRequest build() => _build();

  _$CreateBindingRequest _build() {
    final _$result = _$v ??
        _$CreateBindingRequest._(
          interfaceInstanceId: BuiltValueNullFieldError.checkNotNull(
              interfaceInstanceId,
              r'CreateBindingRequest',
              'interfaceInstanceId'),
          personaId: BuiltValueNullFieldError.checkNotNull(
              personaId, r'CreateBindingRequest', 'personaId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
