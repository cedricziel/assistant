//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'branch_info.g.dart';

/// Where a message sits among its siblings.
///
/// Properties:
/// * [count] - How many versions there are (always at least 2).
/// * [index] - Zero-based position of the active version, in creation order.
/// * [siblingIds] - The first message of every version, in creation order. Switching to any of them selects that version.
@BuiltValue()
abstract class BranchInfo implements Built<BranchInfo, BranchInfoBuilder> {
  /// How many versions there are (always at least 2).
  @BuiltValueField(wireName: r'count')
  int get count;

  /// Zero-based position of the active version, in creation order.
  @BuiltValueField(wireName: r'index')
  int get index;

  /// The first message of every version, in creation order. Switching to any of them selects that version.
  @BuiltValueField(wireName: r'sibling_ids')
  BuiltList<String> get siblingIds;

  BranchInfo._();

  factory BranchInfo([void updates(BranchInfoBuilder b)]) = _$BranchInfo;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(BranchInfoBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<BranchInfo> get serializer => _$BranchInfoSerializer();
}

class _$BranchInfoSerializer implements PrimitiveSerializer<BranchInfo> {
  @override
  final Iterable<Type> types = const [BranchInfo, _$BranchInfo];

  @override
  final String wireName = r'BranchInfo';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    BranchInfo object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'count';
    yield serializers.serialize(
      object.count,
      specifiedType: const FullType(int),
    );
    yield r'index';
    yield serializers.serialize(
      object.index,
      specifiedType: const FullType(int),
    );
    yield r'sibling_ids';
    yield serializers.serialize(
      object.siblingIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    BranchInfo object, {
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
    required BranchInfoBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.count = valueDes;
          break;
        case r'index':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.index = valueDes;
          break;
        case r'sibling_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.siblingIds.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  BranchInfo deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = BranchInfoBuilder();
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
