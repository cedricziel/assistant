// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mcp_app_permissions_view.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$McpAppPermissionsView extends McpAppPermissionsView {
  @override
  final bool camera;
  @override
  final bool clipboardWrite;
  @override
  final bool geolocation;
  @override
  final bool microphone;

  factory _$McpAppPermissionsView(
          [void Function(McpAppPermissionsViewBuilder)? updates]) =>
      (McpAppPermissionsViewBuilder()..update(updates))._build();

  _$McpAppPermissionsView._(
      {required this.camera,
      required this.clipboardWrite,
      required this.geolocation,
      required this.microphone})
      : super._();
  @override
  McpAppPermissionsView rebuild(
          void Function(McpAppPermissionsViewBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  McpAppPermissionsViewBuilder toBuilder() =>
      McpAppPermissionsViewBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is McpAppPermissionsView &&
        camera == other.camera &&
        clipboardWrite == other.clipboardWrite &&
        geolocation == other.geolocation &&
        microphone == other.microphone;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, camera.hashCode);
    _$hash = $jc(_$hash, clipboardWrite.hashCode);
    _$hash = $jc(_$hash, geolocation.hashCode);
    _$hash = $jc(_$hash, microphone.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'McpAppPermissionsView')
          ..add('camera', camera)
          ..add('clipboardWrite', clipboardWrite)
          ..add('geolocation', geolocation)
          ..add('microphone', microphone))
        .toString();
  }
}

class McpAppPermissionsViewBuilder
    implements Builder<McpAppPermissionsView, McpAppPermissionsViewBuilder> {
  _$McpAppPermissionsView? _$v;

  bool? _camera;
  bool? get camera => _$this._camera;
  set camera(bool? camera) => _$this._camera = camera;

  bool? _clipboardWrite;
  bool? get clipboardWrite => _$this._clipboardWrite;
  set clipboardWrite(bool? clipboardWrite) =>
      _$this._clipboardWrite = clipboardWrite;

  bool? _geolocation;
  bool? get geolocation => _$this._geolocation;
  set geolocation(bool? geolocation) => _$this._geolocation = geolocation;

  bool? _microphone;
  bool? get microphone => _$this._microphone;
  set microphone(bool? microphone) => _$this._microphone = microphone;

  McpAppPermissionsViewBuilder() {
    McpAppPermissionsView._defaults(this);
  }

  McpAppPermissionsViewBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _camera = $v.camera;
      _clipboardWrite = $v.clipboardWrite;
      _geolocation = $v.geolocation;
      _microphone = $v.microphone;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(McpAppPermissionsView other) {
    _$v = other as _$McpAppPermissionsView;
  }

  @override
  void update(void Function(McpAppPermissionsViewBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  McpAppPermissionsView build() => _build();

  _$McpAppPermissionsView _build() {
    final _$result = _$v ??
        _$McpAppPermissionsView._(
          camera: BuiltValueNullFieldError.checkNotNull(
              camera, r'McpAppPermissionsView', 'camera'),
          clipboardWrite: BuiltValueNullFieldError.checkNotNull(
              clipboardWrite, r'McpAppPermissionsView', 'clipboardWrite'),
          geolocation: BuiltValueNullFieldError.checkNotNull(
              geolocation, r'McpAppPermissionsView', 'geolocation'),
          microphone: BuiltValueNullFieldError.checkNotNull(
              microphone, r'McpAppPermissionsView', 'microphone'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
