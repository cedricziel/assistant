//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'credential_mode.g.dart';

class CredentialMode extends EnumClass {
  /// Whose credential an integration uses.
  @BuiltValueEnumConst(wireName: r'shared')
  static const CredentialMode shared = _$shared;

  /// Whose credential an integration uses.
  @BuiltValueEnumConst(wireName: r'per_user')
  static const CredentialMode perUser = _$perUser;

  /// Whose credential an integration uses.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const CredentialMode unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<CredentialMode> get serializer =>
      _$credentialModeSerializer;

  const CredentialMode._(String name) : super(name);

  static BuiltSet<CredentialMode> get values => _$values;
  static CredentialMode valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class CredentialModeMixin = Object with _$CredentialModeMixin;
