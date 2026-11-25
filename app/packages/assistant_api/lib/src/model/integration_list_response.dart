//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/integration_summary.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'integration_list_response.g.dart';

/// The integrations the caller may see.
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class IntegrationListResponse
    implements Built<IntegrationListResponse, IntegrationListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<IntegrationSummary> get items;

  IntegrationListResponse._();

  factory IntegrationListResponse(
          [void updates(IntegrationListResponseBuilder b)]) =
      _$IntegrationListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(IntegrationListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<IntegrationListResponse> get serializer =>
      _$IntegrationListResponseSerializer();
}

class _$IntegrationListResponseSerializer
    implements PrimitiveSerializer<IntegrationListResponse> {
  @override
  final Iterable<Type> types = const [
    IntegrationListResponse,
    _$IntegrationListResponse
  ];

  @override
  final String wireName = r'IntegrationListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    IntegrationListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(IntegrationSummary)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    IntegrationListResponse object, {
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
    required IntegrationListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(IntegrationSummary)]),
          ) as BuiltList<IntegrationSummary>;
          result.items.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  IntegrationListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = IntegrationListResponseBuilder();
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
