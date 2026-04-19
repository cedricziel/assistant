//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'time_series_response.g.dart';

/// A single point in a time series.
///
/// Properties:
/// * [bucket] 
/// * [value] 
@BuiltValue()
abstract class TimeSeriesResponse implements Built<TimeSeriesResponse, TimeSeriesResponseBuilder> {
  @BuiltValueField(wireName: r'bucket')
  String get bucket;

  @BuiltValueField(wireName: r'value')
  double get value;

  TimeSeriesResponse._();

  factory TimeSeriesResponse([void updates(TimeSeriesResponseBuilder b)]) = _$TimeSeriesResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(TimeSeriesResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<TimeSeriesResponse> get serializer => _$TimeSeriesResponseSerializer();
}

class _$TimeSeriesResponseSerializer implements PrimitiveSerializer<TimeSeriesResponse> {
  @override
  final Iterable<Type> types = const [TimeSeriesResponse, _$TimeSeriesResponse];

  @override
  final String wireName = r'TimeSeriesResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    TimeSeriesResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'bucket';
    yield serializers.serialize(
      object.bucket,
      specifiedType: const FullType(String),
    );
    yield r'value';
    yield serializers.serialize(
      object.value,
      specifiedType: const FullType(double),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    TimeSeriesResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required TimeSeriesResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'bucket':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.bucket = valueDes;
          break;
        case r'value':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(double),
          ) as double;
          result.value = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  TimeSeriesResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = TimeSeriesResponseBuilder();
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

