//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'command_arg_response.g.dart';

/// A command argument definition for the autocomplete UI.
///
/// Properties:
/// * [completionsEndpoint] - Optional API endpoint that returns completions for this argument.
/// * [name] - Argument name shown in autocomplete.
/// * [required_] - Whether this argument is required.
@BuiltValue()
abstract class CommandArgResponse implements Built<CommandArgResponse, CommandArgResponseBuilder> {
  /// Optional API endpoint that returns completions for this argument.
  @BuiltValueField(wireName: r'completions_endpoint')
  String? get completionsEndpoint;

  /// Argument name shown in autocomplete.
  @BuiltValueField(wireName: r'name')
  String get name;

  /// Whether this argument is required.
  @BuiltValueField(wireName: r'required')
  bool get required_;

  CommandArgResponse._();

  factory CommandArgResponse([void updates(CommandArgResponseBuilder b)]) = _$CommandArgResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CommandArgResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CommandArgResponse> get serializer => _$CommandArgResponseSerializer();
}

class _$CommandArgResponseSerializer implements PrimitiveSerializer<CommandArgResponse> {
  @override
  final Iterable<Type> types = const [CommandArgResponse, _$CommandArgResponse];

  @override
  final String wireName = r'CommandArgResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CommandArgResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.completionsEndpoint != null) {
      yield r'completions_endpoint';
      yield serializers.serialize(
        object.completionsEndpoint,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'required';
    yield serializers.serialize(
      object.required_,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CommandArgResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CommandArgResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'completions_endpoint':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.completionsEndpoint = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'required':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.required_ = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CommandArgResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CommandArgResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}

