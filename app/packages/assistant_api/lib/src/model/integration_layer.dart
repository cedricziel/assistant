//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_layer.g.dart';

class IntegrationLayer extends EnumClass {
  /// Who owns an integration.
  @BuiltValueEnumConst(wireName: r'org')
  static const IntegrationLayer org = _$org;

  /// Who owns an integration.
  @BuiltValueEnumConst(wireName: r'space')
  static const IntegrationLayer space = _$space;

  /// Who owns an integration.
  @BuiltValueEnumConst(wireName: r'user')
  static const IntegrationLayer user = _$user;

  /// Who owns an integration.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const IntegrationLayer unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<IntegrationLayer> get serializer =>
      _$integrationLayerSerializer;

  const IntegrationLayer._(String name) : super(name);

  static BuiltSet<IntegrationLayer> get values => _$values;
  static IntegrationLayer valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class IntegrationLayerMixin = Object with _$IntegrationLayerMixin;
