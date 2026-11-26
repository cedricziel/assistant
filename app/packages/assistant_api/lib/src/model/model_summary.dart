//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'model_summary.g.dart';

/// One model offered for a regenerated reply.
///
/// Properties:
/// * [isDefault] - Whether this is the org's default model.
/// * [name] - The model name, as the provider knows it.
@BuiltValue()
abstract class ModelSummary
    implements Built<ModelSummary, ModelSummaryBuilder> {
  /// Whether this is the org's default model.
  @BuiltValueField(wireName: r'is_default')
  bool get isDefault;

  /// The model name, as the provider knows it.
  @BuiltValueField(wireName: r'name')
  String get name;

  ModelSummary._();

  factory ModelSummary([void updates(ModelSummaryBuilder b)]) = _$ModelSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ModelSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ModelSummary> get serializer => _$ModelSummarySerializer();
}

class _$ModelSummarySerializer implements PrimitiveSerializer<ModelSummary> {
  @override
  final Iterable<Type> types = const [ModelSummary, _$ModelSummary];

  @override
  final String wireName = r'ModelSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ModelSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'is_default';
    yield serializers.serialize(
      object.isDefault,
      specifiedType: const FullType(bool),
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
    ModelSummary object, {
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
    required ModelSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'is_default':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.isDefault = valueDes;
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
  ModelSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ModelSummaryBuilder();
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
