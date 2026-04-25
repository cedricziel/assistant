// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_interface_instance_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateInterfaceInstanceRequest extends CreateInterfaceInstanceRequest {
  @override
  final JsonObject? config;
  @override
  final String interfaceType;

  factory _$CreateInterfaceInstanceRequest(
          [void Function(CreateInterfaceInstanceRequestBuilder)? updates]) =>
      (CreateInterfaceInstanceRequestBuilder()..update(updates))._build();

  _$CreateInterfaceInstanceRequest._({this.config, required this.interfaceType})
      : super._();
  @override
  CreateInterfaceInstanceRequest rebuild(
          void Function(CreateInterfaceInstanceRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateInterfaceInstanceRequestBuilder toBuilder() =>
      CreateInterfaceInstanceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateInterfaceInstanceRequest &&
        config == other.config &&
        interfaceType == other.interfaceType;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, config.hashCode);
    _$hash = $jc(_$hash, interfaceType.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'CreateInterfaceInstanceRequest')
          ..add('config', config)
          ..add('interfaceType', interfaceType))
        .toString();
  }
}

class CreateInterfaceInstanceRequestBuilder
    implements
        Builder<CreateInterfaceInstanceRequest,
            CreateInterfaceInstanceRequestBuilder> {
  _$CreateInterfaceInstanceRequest? _$v;

  JsonObject? _config;
  JsonObject? get config => _$this._config;
  set config(JsonObject? config) => _$this._config = config;

  String? _interfaceType;
  String? get interfaceType => _$this._interfaceType;
  set interfaceType(String? interfaceType) =>
      _$this._interfaceType = interfaceType;

  CreateInterfaceInstanceRequestBuilder() {
    CreateInterfaceInstanceRequest._defaults(this);
  }

  CreateInterfaceInstanceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _config = $v.config;
      _interfaceType = $v.interfaceType;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateInterfaceInstanceRequest other) {
    _$v = other as _$CreateInterfaceInstanceRequest;
  }

  @override
  void update(void Function(CreateInterfaceInstanceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateInterfaceInstanceRequest build() => _build();

  _$CreateInterfaceInstanceRequest _build() {
    final _$result = _$v ??
        _$CreateInterfaceInstanceRequest._(
          config: config,
          interfaceType: BuiltValueNullFieldError.checkNotNull(interfaceType,
              r'CreateInterfaceInstanceRequest', 'interfaceType'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
