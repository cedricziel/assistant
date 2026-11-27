//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/json_object.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'surface_action_request.g.dart';

/// The action a user took on an A2UI surface, after the turn that rendered it ended (design D8).
///
/// Properties:
/// * [context] - The action's context payload.
/// * [dataModel] - The surface's data model at the time of the action.
/// * [name] - The action name, which the surface must declare on a Button.
/// * [sourceComponentId] - Id of the Button component that fired the action.
/// * [surfaceId] - The surface the action belongs to.
@BuiltValue()
abstract class SurfaceActionRequest
    implements Built<SurfaceActionRequest, SurfaceActionRequestBuilder> {
  /// The action's context payload.
  @BuiltValueField(wireName: r'context')
  JsonObject get context;

  /// The surface's data model at the time of the action.
  @BuiltValueField(wireName: r'data_model')
  JsonObject? get dataModel;

  /// The action name, which the surface must declare on a Button.
  @BuiltValueField(wireName: r'name')
  String get name;

  /// Id of the Button component that fired the action.
  @BuiltValueField(wireName: r'source_component_id')
  String get sourceComponentId;

  /// The surface the action belongs to.
  @BuiltValueField(wireName: r'surface_id')
  String get surfaceId;

  SurfaceActionRequest._();

  factory SurfaceActionRequest([void updates(SurfaceActionRequestBuilder b)]) =
      _$SurfaceActionRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SurfaceActionRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SurfaceActionRequest> get serializer =>
      _$SurfaceActionRequestSerializer();
}

class _$SurfaceActionRequestSerializer
    implements PrimitiveSerializer<SurfaceActionRequest> {
  @override
  final Iterable<Type> types = const [
    SurfaceActionRequest,
    _$SurfaceActionRequest
  ];

  @override
  final String wireName = r'SurfaceActionRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SurfaceActionRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'context';
    yield serializers.serialize(
      object.context,
      specifiedType: const FullType(JsonObject),
    );
    if (object.dataModel != null) {
      yield r'data_model';
      yield serializers.serialize(
        object.dataModel,
        specifiedType: const FullType.nullable(JsonObject),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'source_component_id';
    yield serializers.serialize(
      object.sourceComponentId,
      specifiedType: const FullType(String),
    );
    yield r'surface_id';
    yield serializers.serialize(
      object.surfaceId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SurfaceActionRequest object, {
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
    required SurfaceActionRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'context':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(JsonObject),
          ) as JsonObject;
          result.context = valueDes;
          break;
        case r'data_model':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(JsonObject),
          ) as JsonObject?;
          if (valueDes == null) continue;
          result.dataModel = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'source_component_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.sourceComponentId = valueDes;
          break;
        case r'surface_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.surfaceId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SurfaceActionRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SurfaceActionRequestBuilder();
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
