//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'catalog_mode.g.dart';

class CatalogMode extends EnumClass {
  /// Which catalog entries lower layers may install.
  @BuiltValueEnumConst(wireName: r'all')
  static const CatalogMode all = _$all;

  /// Which catalog entries lower layers may install.
  @BuiltValueEnumConst(wireName: r'allowlist')
  static const CatalogMode allowlist = _$allowlist;

  /// Which catalog entries lower layers may install.
  @BuiltValueEnumConst(wireName: r'none')
  static const CatalogMode none = _$none;

  /// Which catalog entries lower layers may install.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const CatalogMode unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<CatalogMode> get serializer => _$catalogModeSerializer;

  const CatalogMode._(String name) : super(name);

  static BuiltSet<CatalogMode> get values => _$values;
  static CatalogMode valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class CatalogModeMixin = Object with _$CatalogModeMixin;
