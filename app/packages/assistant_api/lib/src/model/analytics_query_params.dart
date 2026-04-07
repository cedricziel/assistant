//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'analytics_query_params.g.dart';

/// Query parameters for the analytics endpoint.
///
/// Properties:
/// * [window] - Window in hours. Valid values: 1, 6, 24, 72, 168. Defaults to 24.
@BuiltValue()
abstract class AnalyticsQueryParams implements Built<AnalyticsQueryParams, AnalyticsQueryParamsBuilder> {
  /// Window in hours. Valid values: 1, 6, 24, 72, 168. Defaults to 24.
  @BuiltValueField(wireName: r'window')
  int? get window;

  AnalyticsQueryParams._();

  factory AnalyticsQueryParams([void updates(AnalyticsQueryParamsBuilder b)]) = _$AnalyticsQueryParams;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(AnalyticsQueryParamsBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<AnalyticsQueryParams> get serializer => _$AnalyticsQueryParamsSerializer();
}

class _$AnalyticsQueryParamsSerializer implements PrimitiveSerializer<AnalyticsQueryParams> {
  @override
  final Iterable<Type> types = const [AnalyticsQueryParams, _$AnalyticsQueryParams];

  @override
  final String wireName = r'AnalyticsQueryParams';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    AnalyticsQueryParams object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.window != null) {
      yield r'window';
      yield serializers.serialize(
        object.window,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    AnalyticsQueryParams object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required AnalyticsQueryParamsBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'window':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.window = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  AnalyticsQueryParams deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = AnalyticsQueryParamsBuilder();
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

