// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'surface_action_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SurfaceActionRequest extends SurfaceActionRequest {
  @override
  final JsonObject context;
  @override
  final JsonObject? dataModel;
  @override
  final String name;
  @override
  final String sourceComponentId;
  @override
  final String surfaceId;

  factory _$SurfaceActionRequest(
          [void Function(SurfaceActionRequestBuilder)? updates]) =>
      (SurfaceActionRequestBuilder()..update(updates))._build();

  _$SurfaceActionRequest._(
      {required this.context,
      this.dataModel,
      required this.name,
      required this.sourceComponentId,
      required this.surfaceId})
      : super._();
  @override
  SurfaceActionRequest rebuild(
          void Function(SurfaceActionRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SurfaceActionRequestBuilder toBuilder() =>
      SurfaceActionRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SurfaceActionRequest &&
        context == other.context &&
        dataModel == other.dataModel &&
        name == other.name &&
        sourceComponentId == other.sourceComponentId &&
        surfaceId == other.surfaceId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, context.hashCode);
    _$hash = $jc(_$hash, dataModel.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, sourceComponentId.hashCode);
    _$hash = $jc(_$hash, surfaceId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SurfaceActionRequest')
          ..add('context', context)
          ..add('dataModel', dataModel)
          ..add('name', name)
          ..add('sourceComponentId', sourceComponentId)
          ..add('surfaceId', surfaceId))
        .toString();
  }
}

class SurfaceActionRequestBuilder
    implements Builder<SurfaceActionRequest, SurfaceActionRequestBuilder> {
  _$SurfaceActionRequest? _$v;

  JsonObject? _context;
  JsonObject? get context => _$this._context;
  set context(JsonObject? context) => _$this._context = context;

  JsonObject? _dataModel;
  JsonObject? get dataModel => _$this._dataModel;
  set dataModel(JsonObject? dataModel) => _$this._dataModel = dataModel;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _sourceComponentId;
  String? get sourceComponentId => _$this._sourceComponentId;
  set sourceComponentId(String? sourceComponentId) =>
      _$this._sourceComponentId = sourceComponentId;

  String? _surfaceId;
  String? get surfaceId => _$this._surfaceId;
  set surfaceId(String? surfaceId) => _$this._surfaceId = surfaceId;

  SurfaceActionRequestBuilder() {
    SurfaceActionRequest._defaults(this);
  }

  SurfaceActionRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _context = $v.context;
      _dataModel = $v.dataModel;
      _name = $v.name;
      _sourceComponentId = $v.sourceComponentId;
      _surfaceId = $v.surfaceId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SurfaceActionRequest other) {
    _$v = other as _$SurfaceActionRequest;
  }

  @override
  void update(void Function(SurfaceActionRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SurfaceActionRequest build() => _build();

  _$SurfaceActionRequest _build() {
    final _$result = _$v ??
        _$SurfaceActionRequest._(
          context: BuiltValueNullFieldError.checkNotNull(
              context, r'SurfaceActionRequest', 'context'),
          dataModel: dataModel,
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'SurfaceActionRequest', 'name'),
          sourceComponentId: BuiltValueNullFieldError.checkNotNull(
              sourceComponentId, r'SurfaceActionRequest', 'sourceComponentId'),
          surfaceId: BuiltValueNullFieldError.checkNotNull(
              surfaceId, r'SurfaceActionRequest', 'surfaceId'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
