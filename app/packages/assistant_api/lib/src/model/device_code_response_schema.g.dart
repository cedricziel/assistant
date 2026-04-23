// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'device_code_response_schema.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeviceCodeResponseSchema extends DeviceCodeResponseSchema {
  @override
  final String deviceCode;
  @override
  final int expiresIn;
  @override
  final int interval;
  @override
  final String userCode;
  @override
  final String verificationUri;

  factory _$DeviceCodeResponseSchema(
          [void Function(DeviceCodeResponseSchemaBuilder)? updates]) =>
      (DeviceCodeResponseSchemaBuilder()..update(updates))._build();

  _$DeviceCodeResponseSchema._(
      {required this.deviceCode,
      required this.expiresIn,
      required this.interval,
      required this.userCode,
      required this.verificationUri})
      : super._();
  @override
  DeviceCodeResponseSchema rebuild(
          void Function(DeviceCodeResponseSchemaBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  DeviceCodeResponseSchemaBuilder toBuilder() =>
      DeviceCodeResponseSchemaBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeviceCodeResponseSchema &&
        deviceCode == other.deviceCode &&
        expiresIn == other.expiresIn &&
        interval == other.interval &&
        userCode == other.userCode &&
        verificationUri == other.verificationUri;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, deviceCode.hashCode);
    _$hash = $jc(_$hash, expiresIn.hashCode);
    _$hash = $jc(_$hash, interval.hashCode);
    _$hash = $jc(_$hash, userCode.hashCode);
    _$hash = $jc(_$hash, verificationUri.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeviceCodeResponseSchema')
          ..add('deviceCode', deviceCode)
          ..add('expiresIn', expiresIn)
          ..add('interval', interval)
          ..add('userCode', userCode)
          ..add('verificationUri', verificationUri))
        .toString();
  }
}

class DeviceCodeResponseSchemaBuilder
    implements
        Builder<DeviceCodeResponseSchema, DeviceCodeResponseSchemaBuilder> {
  _$DeviceCodeResponseSchema? _$v;

  String? _deviceCode;
  String? get deviceCode => _$this._deviceCode;
  set deviceCode(String? deviceCode) => _$this._deviceCode = deviceCode;

  int? _expiresIn;
  int? get expiresIn => _$this._expiresIn;
  set expiresIn(int? expiresIn) => _$this._expiresIn = expiresIn;

  int? _interval;
  int? get interval => _$this._interval;
  set interval(int? interval) => _$this._interval = interval;

  String? _userCode;
  String? get userCode => _$this._userCode;
  set userCode(String? userCode) => _$this._userCode = userCode;

  String? _verificationUri;
  String? get verificationUri => _$this._verificationUri;
  set verificationUri(String? verificationUri) =>
      _$this._verificationUri = verificationUri;

  DeviceCodeResponseSchemaBuilder() {
    DeviceCodeResponseSchema._defaults(this);
  }

  DeviceCodeResponseSchemaBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _deviceCode = $v.deviceCode;
      _expiresIn = $v.expiresIn;
      _interval = $v.interval;
      _userCode = $v.userCode;
      _verificationUri = $v.verificationUri;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeviceCodeResponseSchema other) {
    _$v = other as _$DeviceCodeResponseSchema;
  }

  @override
  void update(void Function(DeviceCodeResponseSchemaBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeviceCodeResponseSchema build() => _build();

  _$DeviceCodeResponseSchema _build() {
    final _$result = _$v ??
        _$DeviceCodeResponseSchema._(
          deviceCode: BuiltValueNullFieldError.checkNotNull(
              deviceCode, r'DeviceCodeResponseSchema', 'deviceCode'),
          expiresIn: BuiltValueNullFieldError.checkNotNull(
              expiresIn, r'DeviceCodeResponseSchema', 'expiresIn'),
          interval: BuiltValueNullFieldError.checkNotNull(
              interval, r'DeviceCodeResponseSchema', 'interval'),
          userCode: BuiltValueNullFieldError.checkNotNull(
              userCode, r'DeviceCodeResponseSchema', 'userCode'),
          verificationUri: BuiltValueNullFieldError.checkNotNull(
              verificationUri, r'DeviceCodeResponseSchema', 'verificationUri'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
