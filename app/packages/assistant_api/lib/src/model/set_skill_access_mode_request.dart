//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_skill_access_mode_request.g.dart';

/// Body for `PATCH /api/personas/{id}/skill-access`.
///
/// Properties:
/// * [mode] - One of \"all\", \"whitelist\", or \"blacklist\".
@BuiltValue()
abstract class SetSkillAccessModeRequest
    implements
        Built<SetSkillAccessModeRequest, SetSkillAccessModeRequestBuilder> {
  /// One of \"all\", \"whitelist\", or \"blacklist\".
  @BuiltValueField(wireName: r'mode')
  String get mode;

  SetSkillAccessModeRequest._();

  factory SetSkillAccessModeRequest(
          [void updates(SetSkillAccessModeRequestBuilder b)]) =
      _$SetSkillAccessModeRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetSkillAccessModeRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetSkillAccessModeRequest> get serializer =>
      _$SetSkillAccessModeRequestSerializer();
}

class _$SetSkillAccessModeRequestSerializer
    implements PrimitiveSerializer<SetSkillAccessModeRequest> {
  @override
  final Iterable<Type> types = const [
    SetSkillAccessModeRequest,
    _$SetSkillAccessModeRequest
  ];

  @override
  final String wireName = r'SetSkillAccessModeRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetSkillAccessModeRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'mode';
    yield serializers.serialize(
      object.mode,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetSkillAccessModeRequest object, {
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
    required SetSkillAccessModeRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.mode = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetSkillAccessModeRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetSkillAccessModeRequestBuilder();
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
