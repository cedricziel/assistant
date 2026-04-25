// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'onboarding_status_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$OnboardingStatusResponse extends OnboardingStatusResponse {
  @override
  final bool hasPersona;

  factory _$OnboardingStatusResponse(
          [void Function(OnboardingStatusResponseBuilder)? updates]) =>
      (OnboardingStatusResponseBuilder()..update(updates))._build();

  _$OnboardingStatusResponse._({required this.hasPersona}) : super._();
  @override
  OnboardingStatusResponse rebuild(
          void Function(OnboardingStatusResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  OnboardingStatusResponseBuilder toBuilder() =>
      OnboardingStatusResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is OnboardingStatusResponse && hasPersona == other.hasPersona;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, hasPersona.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'OnboardingStatusResponse')
          ..add('hasPersona', hasPersona))
        .toString();
  }
}

class OnboardingStatusResponseBuilder
    implements
        Builder<OnboardingStatusResponse, OnboardingStatusResponseBuilder> {
  _$OnboardingStatusResponse? _$v;

  bool? _hasPersona;
  bool? get hasPersona => _$this._hasPersona;
  set hasPersona(bool? hasPersona) => _$this._hasPersona = hasPersona;

  OnboardingStatusResponseBuilder() {
    OnboardingStatusResponse._defaults(this);
  }

  OnboardingStatusResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _hasPersona = $v.hasPersona;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(OnboardingStatusResponse other) {
    _$v = other as _$OnboardingStatusResponse;
  }

  @override
  void update(void Function(OnboardingStatusResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  OnboardingStatusResponse build() => _build();

  _$OnboardingStatusResponse _build() {
    final _$result = _$v ??
        _$OnboardingStatusResponse._(
          hasPersona: BuiltValueNullFieldError.checkNotNull(
              hasPersona, r'OnboardingStatusResponse', 'hasPersona'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
