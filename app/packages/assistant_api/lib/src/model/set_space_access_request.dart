//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/space_access_mode.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'set_space_access_request.g.dart';

/// Body of `PUT /api/integrations/{id}/spaces/{space_id}`.
///
/// Properties:
/// * [mode]
@BuiltValue()
abstract class SetSpaceAccessRequest
    implements Built<SetSpaceAccessRequest, SetSpaceAccessRequestBuilder> {
  @BuiltValueField(wireName: r'mode')
  SpaceAccessMode get mode;
  // enum modeEnum {  allow,  opt_out,  };

  SetSpaceAccessRequest._();

  factory SetSpaceAccessRequest(
      [void updates(SetSpaceAccessRequestBuilder b)]) = _$SetSpaceAccessRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetSpaceAccessRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetSpaceAccessRequest> get serializer =>
      _$SetSpaceAccessRequestSerializer();
}

class _$SetSpaceAccessRequestSerializer
    implements PrimitiveSerializer<SetSpaceAccessRequest> {
  @override
  final Iterable<Type> types = const [
    SetSpaceAccessRequest,
    _$SetSpaceAccessRequest
  ];

  @override
  final String wireName = r'SetSpaceAccessRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetSpaceAccessRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'mode';
    yield serializers.serialize(
      object.mode,
      specifiedType: const FullType(SpaceAccessMode),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetSpaceAccessRequest object, {
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
    required SetSpaceAccessRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(SpaceAccessMode),
          ) as SpaceAccessMode;
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
  SetSpaceAccessRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetSpaceAccessRequestBuilder();
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
