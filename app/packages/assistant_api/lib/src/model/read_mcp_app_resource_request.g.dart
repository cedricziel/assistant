// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'read_mcp_app_resource_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ReadMcpAppResourceRequest extends ReadMcpAppResourceRequest {
  @override
  final String uri;

  factory _$ReadMcpAppResourceRequest(
          [void Function(ReadMcpAppResourceRequestBuilder)? updates]) =>
      (ReadMcpAppResourceRequestBuilder()..update(updates))._build();

  _$ReadMcpAppResourceRequest._({required this.uri}) : super._();
  @override
  ReadMcpAppResourceRequest rebuild(
          void Function(ReadMcpAppResourceRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ReadMcpAppResourceRequestBuilder toBuilder() =>
      ReadMcpAppResourceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ReadMcpAppResourceRequest && uri == other.uri;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, uri.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ReadMcpAppResourceRequest')
          ..add('uri', uri))
        .toString();
  }
}

class ReadMcpAppResourceRequestBuilder
    implements
        Builder<ReadMcpAppResourceRequest, ReadMcpAppResourceRequestBuilder> {
  _$ReadMcpAppResourceRequest? _$v;

  String? _uri;
  String? get uri => _$this._uri;
  set uri(String? uri) => _$this._uri = uri;

  ReadMcpAppResourceRequestBuilder() {
    ReadMcpAppResourceRequest._defaults(this);
  }

  ReadMcpAppResourceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _uri = $v.uri;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ReadMcpAppResourceRequest other) {
    _$v = other as _$ReadMcpAppResourceRequest;
  }

  @override
  void update(void Function(ReadMcpAppResourceRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ReadMcpAppResourceRequest build() => _build();

  _$ReadMcpAppResourceRequest _build() {
    final _$result = _$v ??
        _$ReadMcpAppResourceRequest._(
          uri: BuiltValueNullFieldError.checkNotNull(
              uri, r'ReadMcpAppResourceRequest', 'uri'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
