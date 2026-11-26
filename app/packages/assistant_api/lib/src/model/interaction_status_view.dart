//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'interaction_status_view.g.dart';

class InteractionStatusView extends EnumClass {
  /// Lifecycle state of an interaction.
  @BuiltValueEnumConst(wireName: r'pending')
  static const InteractionStatusView pending = _$pending;

  /// Lifecycle state of an interaction.
  @BuiltValueEnumConst(wireName: r'resolved')
  static const InteractionStatusView resolved = _$resolved;

  /// Lifecycle state of an interaction.
  @BuiltValueEnumConst(wireName: r'cancelled')
  static const InteractionStatusView cancelled = _$cancelled;

  /// Lifecycle state of an interaction.
  @BuiltValueEnumConst(wireName: r'expired')
  static const InteractionStatusView expired = _$expired;

  /// Lifecycle state of an interaction.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const InteractionStatusView unknownDefaultOpenApi =
      _$unknownDefaultOpenApi;

  static Serializer<InteractionStatusView> get serializer =>
      _$interactionStatusViewSerializer;

  const InteractionStatusView._(String name) : super(name);

  static BuiltSet<InteractionStatusView> get values => _$values;
  static InteractionStatusView valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class InteractionStatusViewMixin = Object
    with _$InteractionStatusViewMixin;
