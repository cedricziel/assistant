//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'respond_to_interaction_request_one_of2.g.dart';

/// Answers a `surface_action` interaction with an A2UI action the surface declares, and optionally the surface's data model.
///
/// Properties:
/// * [action] - The action object: `name` (a Button action the surface declares), plus `sourceComponentId` and `context` when the client has them.
/// * [dataModel] - The surface's data model at the time of the action.
/// * [kind]
@BuiltValue()
abstract class RespondToInteractionRequestOneOf2
    implements
        Built<RespondToInteractionRequestOneOf2,
            RespondToInteractionRequestOneOf2Builder> {
  /// The action object: `name` (a Button action the surface declares), plus `sourceComponentId` and `context` when the client has them.
  @BuiltValueField(wireName: r'action')
  JsonObject get action;

  /// The surface's data model at the time of the action.
  @BuiltValueField(wireName: r'data_model')
  JsonObject? get dataModel;

  @BuiltValueField(wireName: r'kind')
  RespondToInteractionRequestOneOf2KindEnum get kind;
  // enum kindEnum {  surface_action,  };

  RespondToInteractionRequestOneOf2._();

  factory RespondToInteractionRequestOneOf2(
          [void updates(RespondToInteractionRequestOneOf2Builder b)]) =
      _$RespondToInteractionRequestOneOf2;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RespondToInteractionRequestOneOf2Builder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RespondToInteractionRequestOneOf2> get serializer =>
      _$RespondToInteractionRequestOneOf2Serializer();
}

class _$RespondToInteractionRequestOneOf2Serializer
    implements PrimitiveSerializer<RespondToInteractionRequestOneOf2> {
  @override
  final Iterable<Type> types = const [
    RespondToInteractionRequestOneOf2,
    _$RespondToInteractionRequestOneOf2
  ];

  @override
  final String wireName = r'RespondToInteractionRequestOneOf2';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RespondToInteractionRequestOneOf2 object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'action';
    yield serializers.serialize(
      object.action,
      specifiedType: const FullType(JsonObject),
    );
    if (object.dataModel != null) {
      yield r'data_model';
      yield serializers.serialize(
        object.dataModel,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'kind';
    yield serializers.serialize(
      object.kind,
      specifiedType: const FullType(RespondToInteractionRequestOneOf2KindEnum),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    RespondToInteractionRequestOneOf2 object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object,
            specifiedType: specifiedType)
        .toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RespondToInteractionRequestOneOf2Builder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'action':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(JsonObject),
          ) as JsonObject;
          result.action = valueDes;
          break;
        case r'data_model':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.dataModel = valueDes;
          break;
        case r'kind':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(RespondToInteractionRequestOneOf2KindEnum),
          ) as RespondToInteractionRequestOneOf2KindEnum;
          result.kind = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RespondToInteractionRequestOneOf2 deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RespondToInteractionRequestOneOf2Builder();
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

class RespondToInteractionRequestOneOf2KindEnum extends EnumClass {
  @BuiltValueEnumConst(wireName: r'surface_action')
  static const RespondToInteractionRequestOneOf2KindEnum surfaceAction =
      _$respondToInteractionRequestOneOf2KindEnum_surfaceAction;
  @BuiltValueEnumConst(wireName: r'unknown_default_open_api', fallback: true)
  static const RespondToInteractionRequestOneOf2KindEnum unknownDefaultOpenApi =
      _$respondToInteractionRequestOneOf2KindEnum_unknownDefaultOpenApi;

  static Serializer<RespondToInteractionRequestOneOf2KindEnum> get serializer =>
      _$respondToInteractionRequestOneOf2KindEnumSerializer;

  const RespondToInteractionRequestOneOf2KindEnum._(String name) : super(name);

  static BuiltSet<RespondToInteractionRequestOneOf2KindEnum> get values =>
      _$respondToInteractionRequestOneOf2KindEnumValues;
  static RespondToInteractionRequestOneOf2KindEnum valueOf(String name) =>
      _$respondToInteractionRequestOneOf2KindEnumValueOf(name);
}
