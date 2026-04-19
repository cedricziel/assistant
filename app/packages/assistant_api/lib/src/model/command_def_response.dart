//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/command_arg_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'command_def_response.g.dart';

/// A command definition for the autocomplete UI.
///
/// Properties:
/// * [args] - Argument definitions.
/// * [category] - Grouping category (e.g. `\"session\"`, `\"config\"`, `\"debug\"`).
/// * [description] - One-line description.
/// * [name] - Command name without the leading `/`.
@BuiltValue()
abstract class CommandDefResponse implements Built<CommandDefResponse, CommandDefResponseBuilder> {
  /// Argument definitions.
  @BuiltValueField(wireName: r'args')
  BuiltList<CommandArgResponse> get args;

  /// Grouping category (e.g. `\"session\"`, `\"config\"`, `\"debug\"`).
  @BuiltValueField(wireName: r'category')
  String get category;

  /// One-line description.
  @BuiltValueField(wireName: r'description')
  String get description;

  /// Command name without the leading `/`.
  @BuiltValueField(wireName: r'name')
  String get name;

  CommandDefResponse._();

  factory CommandDefResponse([void updates(CommandDefResponseBuilder b)]) = _$CommandDefResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CommandDefResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CommandDefResponse> get serializer => _$CommandDefResponseSerializer();
}

class _$CommandDefResponseSerializer implements PrimitiveSerializer<CommandDefResponse> {
  @override
  final Iterable<Type> types = const [CommandDefResponse, _$CommandDefResponse];

  @override
  final String wireName = r'CommandDefResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CommandDefResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'args';
    yield serializers.serialize(
      object.args,
      specifiedType: const FullType(BuiltList, [FullType(CommandArgResponse)]),
    );
    yield r'category';
    yield serializers.serialize(
      object.category,
      specifiedType: const FullType(String),
    );
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CommandDefResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CommandDefResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'args':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(CommandArgResponse)]),
          ) as BuiltList<CommandArgResponse>;
          result.args.replace(valueDes);
          break;
        case r'category':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.category = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CommandDefResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CommandDefResponseBuilder();
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

