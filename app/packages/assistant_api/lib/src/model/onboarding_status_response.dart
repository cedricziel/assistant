//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'onboarding_status_response.g.dart';

/// OnboardingStatusResponse
///
/// Properties:
/// * [hasPersona] - Whether the user has created at least one persona.
@BuiltValue()
abstract class OnboardingStatusResponse
    implements
        Built<OnboardingStatusResponse, OnboardingStatusResponseBuilder> {
  /// Whether the user has created at least one persona.
  @BuiltValueField(wireName: r'has_persona')
  bool get hasPersona;

  OnboardingStatusResponse._();

  factory OnboardingStatusResponse(
          [void updates(OnboardingStatusResponseBuilder b)]) =
      _$OnboardingStatusResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OnboardingStatusResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OnboardingStatusResponse> get serializer =>
      _$OnboardingStatusResponseSerializer();
}

class _$OnboardingStatusResponseSerializer
    implements PrimitiveSerializer<OnboardingStatusResponse> {
  @override
  final Iterable<Type> types = const [
    OnboardingStatusResponse,
    _$OnboardingStatusResponse
  ];

  @override
  final String wireName = r'OnboardingStatusResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OnboardingStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'has_persona';
    yield serializers.serialize(
      object.hasPersona,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    OnboardingStatusResponse object, {
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
    required OnboardingStatusResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'has_persona':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.hasPersona = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OnboardingStatusResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OnboardingStatusResponseBuilder();
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
