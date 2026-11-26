//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'switch_branch_request.g.dart';

/// Body for `POST /api/conversations/{id}/switch-branch`.
///
/// Properties:
/// * [messageId] - Any message of the version to switch to. The active path continues from it through the most recently created child at each fork.
@BuiltValue()
abstract class SwitchBranchRequest
    implements Built<SwitchBranchRequest, SwitchBranchRequestBuilder> {
  /// Any message of the version to switch to. The active path continues from it through the most recently created child at each fork.
  @BuiltValueField(wireName: r'message_id')
  String get messageId;

  SwitchBranchRequest._();

  factory SwitchBranchRequest([void updates(SwitchBranchRequestBuilder b)]) =
      _$SwitchBranchRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SwitchBranchRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SwitchBranchRequest> get serializer =>
      _$SwitchBranchRequestSerializer();
}

class _$SwitchBranchRequestSerializer
    implements PrimitiveSerializer<SwitchBranchRequest> {
  @override
  final Iterable<Type> types = const [
    SwitchBranchRequest,
    _$SwitchBranchRequest
  ];

  @override
  final String wireName = r'SwitchBranchRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SwitchBranchRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'message_id';
    yield serializers.serialize(
      object.messageId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SwitchBranchRequest object, {
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
    required SwitchBranchRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.messageId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SwitchBranchRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SwitchBranchRequestBuilder();
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
