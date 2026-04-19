//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'role.g.dart';

class Role extends EnumClass {

  /// Defines the sender of a message in A2A protocol communication.
  @BuiltValueEnumConst(wireName: r'ROLE_UNSPECIFIED')
  static const Role ROLE_UNSPECIFIED = _$ROLE_UNSPECIFIED;
  /// Defines the sender of a message in A2A protocol communication.
  @BuiltValueEnumConst(wireName: r'ROLE_USER')
  static const Role ROLE_USER = _$ROLE_USER;
  /// Defines the sender of a message in A2A protocol communication.
  @BuiltValueEnumConst(wireName: r'ROLE_AGENT')
  static const Role ROLE_AGENT = _$ROLE_AGENT;
  /// Defines the sender of a message in A2A protocol communication.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const Role unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<Role> get serializer => _$roleSerializer;

  const Role._(String name): super(name);

  static BuiltSet<Role> get values => _$values;
  static Role valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class RoleMixin = Object with _$RoleMixin;

