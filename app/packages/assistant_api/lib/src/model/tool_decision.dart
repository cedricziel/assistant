//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tool_decision.g.dart';

class ToolDecision extends EnumClass {
  /// What the owner decided about a tool call.
  @BuiltValueEnumConst(wireName: r'allow_once')
  static const ToolDecision allowOnce = _$allowOnce;

  /// What the owner decided about a tool call.
  @BuiltValueEnumConst(wireName: r'allow_always')
  static const ToolDecision allowAlways = _$allowAlways;

  /// What the owner decided about a tool call.
  @BuiltValueEnumConst(wireName: r'reject_once')
  static const ToolDecision rejectOnce = _$rejectOnce;

  /// What the owner decided about a tool call.
  @BuiltValueEnumConst(wireName: r'reject_always')
  static const ToolDecision rejectAlways = _$rejectAlways;

  /// What the owner decided about a tool call.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const ToolDecision unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<ToolDecision> get serializer => _$toolDecisionSerializer;

  const ToolDecision._(String name) : super(name);

  static BuiltSet<ToolDecision> get values => _$values;
  static ToolDecision valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class ToolDecisionMixin = Object with _$ToolDecisionMixin;
