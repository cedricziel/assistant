// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'get_extended_agent_card_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$GetExtendedAgentCardRequest extends GetExtendedAgentCardRequest {
  @override
  final String? tenant;

  factory _$GetExtendedAgentCardRequest(
          [void Function(GetExtendedAgentCardRequestBuilder)? updates]) =>
      (GetExtendedAgentCardRequestBuilder()..update(updates))._build();

  _$GetExtendedAgentCardRequest._({this.tenant}) : super._();
  @override
  GetExtendedAgentCardRequest rebuild(
          void Function(GetExtendedAgentCardRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  GetExtendedAgentCardRequestBuilder toBuilder() =>
      GetExtendedAgentCardRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is GetExtendedAgentCardRequest && tenant == other.tenant;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, tenant.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'GetExtendedAgentCardRequest')
          ..add('tenant', tenant))
        .toString();
  }
}

class GetExtendedAgentCardRequestBuilder
    implements
        Builder<GetExtendedAgentCardRequest,
            GetExtendedAgentCardRequestBuilder> {
  _$GetExtendedAgentCardRequest? _$v;

  String? _tenant;
  String? get tenant => _$this._tenant;
  set tenant(String? tenant) => _$this._tenant = tenant;

  GetExtendedAgentCardRequestBuilder() {
    GetExtendedAgentCardRequest._defaults(this);
  }

  GetExtendedAgentCardRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _tenant = $v.tenant;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(GetExtendedAgentCardRequest other) {
    _$v = other as _$GetExtendedAgentCardRequest;
  }

  @override
  void update(void Function(GetExtendedAgentCardRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  GetExtendedAgentCardRequest build() => _build();

  _$GetExtendedAgentCardRequest _build() {
    final _$result = _$v ??
        _$GetExtendedAgentCardRequest._(
          tenant: tenant,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
