//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_status.g.dart';

class IntegrationStatus extends EnumClass {
  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'ready')
  static const IntegrationStatus ready = _$ready;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'needs_connection')
  static const IntegrationStatus needsConnection = _$needsConnection;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'auth_expired')
  static const IntegrationStatus authExpired = _$authExpired;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'unreachable')
  static const IntegrationStatus unreachable = _$unreachable;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'blocked_by_policy')
  static const IntegrationStatus blockedByPolicy = _$blockedByPolicy;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'disabled')
  static const IntegrationStatus disabled = _$disabled;

  /// Whether the caller can use an integration right now.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const IntegrationStatus unknownDefaultOpenApi =
      _$unknownDefaultOpenApi;

  static Serializer<IntegrationStatus> get serializer =>
      _$integrationStatusSerializer;

  const IntegrationStatus._(String name) : super(name);

  static BuiltSet<IntegrationStatus> get values => _$values;
  static IntegrationStatus valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class IntegrationStatusMixin = Object with _$IntegrationStatusMixin;
