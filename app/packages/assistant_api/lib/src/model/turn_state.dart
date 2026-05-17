//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'turn_state.g.dart';

class TurnState extends EnumClass {
  /// Closed enum of turn states surfaced by the status endpoint.  Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.
  @BuiltValueEnumConst(wireName: r'running')
  static const TurnState running = _$running;

  /// Closed enum of turn states surfaced by the status endpoint.  Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.
  @BuiltValueEnumConst(wireName: r'completed')
  static const TurnState completed = _$completed;

  /// Closed enum of turn states surfaced by the status endpoint.  Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.
  @BuiltValueEnumConst(wireName: r'errored')
  static const TurnState errored = _$errored;

  /// Closed enum of turn states surfaced by the status endpoint.  Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.
  @BuiltValueEnumConst(wireName: r'unknown')
  static const TurnState unknown = _$unknown;

  /// Closed enum of turn states surfaced by the status endpoint.  Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const TurnState unknownDefaultOpenApi = _$unknownDefaultOpenApi;

  static Serializer<TurnState> get serializer => _$turnStateSerializer;

  const TurnState._(String name) : super(name);

  static BuiltSet<TurnState> get values => _$values;
  static TurnState valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class TurnStateMixin = Object with _$TurnStateMixin;
