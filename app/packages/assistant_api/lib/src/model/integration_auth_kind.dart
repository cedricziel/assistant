//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_auth_kind.g.dart';

class IntegrationAuthKind extends EnumClass {
  /// How an integration authenticates to its MCP server.
  @BuiltValueEnumConst(wireName: r'none')
  static const IntegrationAuthKind none = _$none;

  /// How an integration authenticates to its MCP server.
  @BuiltValueEnumConst(wireName: r'header')
  static const IntegrationAuthKind header = _$header;

  /// How an integration authenticates to its MCP server.
  @BuiltValueEnumConst(wireName: r'oauth')
  static const IntegrationAuthKind oauth = _$oauth;

  /// How an integration authenticates to its MCP server.
  @BuiltValueEnumConst(wireName: r'mixed')
  static const IntegrationAuthKind mixed = _$mixed;

  /// How an integration authenticates to its MCP server.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const IntegrationAuthKind unknownDefaultOpenApi =
      _$unknownDefaultOpenApi;

  static Serializer<IntegrationAuthKind> get serializer =>
      _$integrationAuthKindSerializer;

  const IntegrationAuthKind._(String name) : super(name);

  static BuiltSet<IntegrationAuthKind> get values => _$values;
  static IntegrationAuthKind valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class IntegrationAuthKindMixin = Object
    with _$IntegrationAuthKindMixin;
