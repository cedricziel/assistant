// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ui_capabilities.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UiCapabilities extends UiCapabilities {
  @override
  final BuiltList<String> linkHosts;

  factory _$UiCapabilities([void Function(UiCapabilitiesBuilder)? updates]) =>
      (UiCapabilitiesBuilder()..update(updates))._build();

  _$UiCapabilities._({required this.linkHosts}) : super._();
  @override
  UiCapabilities rebuild(void Function(UiCapabilitiesBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UiCapabilitiesBuilder toBuilder() => UiCapabilitiesBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UiCapabilities && linkHosts == other.linkHosts;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, linkHosts.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UiCapabilities')
          ..add('linkHosts', linkHosts))
        .toString();
  }
}

class UiCapabilitiesBuilder
    implements Builder<UiCapabilities, UiCapabilitiesBuilder> {
  _$UiCapabilities? _$v;

  ListBuilder<String>? _linkHosts;
  ListBuilder<String> get linkHosts =>
      _$this._linkHosts ??= ListBuilder<String>();
  set linkHosts(ListBuilder<String>? linkHosts) =>
      _$this._linkHosts = linkHosts;

  UiCapabilitiesBuilder() {
    UiCapabilities._defaults(this);
  }

  UiCapabilitiesBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _linkHosts = $v.linkHosts.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UiCapabilities other) {
    _$v = other as _$UiCapabilities;
  }

  @override
  void update(void Function(UiCapabilitiesBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UiCapabilities build() => _build();

  _$UiCapabilities _build() {
    _$UiCapabilities _$result;
    try {
      _$result = _$v ??
          _$UiCapabilities._(
            linkHosts: linkHosts.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'linkHosts';
        linkHosts.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
            r'UiCapabilities', _$failedField, e.toString());
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
