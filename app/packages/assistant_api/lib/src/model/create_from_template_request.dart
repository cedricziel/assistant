//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_from_template_request.g.dart';

/// CreateFromTemplateRequest
///
/// Properties:
/// * [name] - Name for the new persona (optional — defaults to template name).
/// * [templateId] - Catalog item ID of the template to instantiate.
@BuiltValue()
abstract class CreateFromTemplateRequest
    implements
        Built<CreateFromTemplateRequest, CreateFromTemplateRequestBuilder> {
  /// Name for the new persona (optional — defaults to template name).
  @BuiltValueField(wireName: r'name')
  String? get name;

  /// Catalog item ID of the template to instantiate.
  @BuiltValueField(wireName: r'template_id')
  String get templateId;

  CreateFromTemplateRequest._();

  factory CreateFromTemplateRequest(
          [void updates(CreateFromTemplateRequestBuilder b)]) =
      _$CreateFromTemplateRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateFromTemplateRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateFromTemplateRequest> get serializer =>
      _$CreateFromTemplateRequestSerializer();
}

class _$CreateFromTemplateRequestSerializer
    implements PrimitiveSerializer<CreateFromTemplateRequest> {
  @override
  final Iterable<Type> types = const [
    CreateFromTemplateRequest,
    _$CreateFromTemplateRequest
  ];

  @override
  final String wireName = r'CreateFromTemplateRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateFromTemplateRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.name != null) {
      yield r'name';
      yield serializers.serialize(
        object.name,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'template_id';
    yield serializers.serialize(
      object.templateId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateFromTemplateRequest object, {
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
    required CreateFromTemplateRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.name = valueDes;
          break;
        case r'template_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.templateId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateFromTemplateRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateFromTemplateRequestBuilder();
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
