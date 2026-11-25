//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_trust.g.dart';

class IntegrationTrust extends EnumClass {
  /// Whether tool calls need user confirmation.
  @BuiltValueEnumConst(wireName: r'confirm')
  static const IntegrationTrust confirm = _$confirm;

  /// Whether tool calls need user confirmation.
  @BuiltValueEnumConst(wireName: r'trust')
  static const IntegrationTrust trust = _$trust;

  /// Whether tool calls need user confirmation.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const IntegrationTrust unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<IntegrationTrust> get serializer =>
      _$integrationTrustSerializer;

  const IntegrationTrust._(String name) : super(name);

  static BuiltSet<IntegrationTrust> get values => _$values;
  static IntegrationTrust valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class IntegrationTrustMixin = Object with _$IntegrationTrustMixin;
