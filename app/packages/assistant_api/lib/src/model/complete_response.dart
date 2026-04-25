//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'complete_response.g.dart';

/// JSON response from the completion endpoint.
///
/// Properties:
/// * [code] - The authorization code.
/// * [state] - The client state, if any.
@BuiltValue()
abstract class CompleteResponse
    implements Built<CompleteResponse, CompleteResponseBuilder> {
  /// The authorization code.
  @BuiltValueField(wireName: r'code')
  String get code;

  /// The client state, if any.
  @BuiltValueField(wireName: r'state')
  String? get state;

  CompleteResponse._();

  factory CompleteResponse([void updates(CompleteResponseBuilder b)]) =
      _$CompleteResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CompleteResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CompleteResponse> get serializer =>
      _$CompleteResponseSerializer();
}

class _$CompleteResponseSerializer
    implements PrimitiveSerializer<CompleteResponse> {
  @override
  final Iterable<Type> types = const [CompleteResponse, _$CompleteResponse];

  @override
  final String wireName = r'CompleteResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CompleteResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'code';
    yield serializers.serialize(
      object.code,
      specifiedType: const FullType(String),
    );
    if (object.state != null) {
      yield r'state';
      yield serializers.serialize(
        object.state,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CompleteResponse object, {
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
    required CompleteResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.code = valueDes;
          break;
        case r'state':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.state = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CompleteResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CompleteResponseBuilder();
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
