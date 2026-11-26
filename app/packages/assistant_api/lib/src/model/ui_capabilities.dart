//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'ui_capabilities.g.dart';

/// The part of the org's `[ui]` settings a client needs.  `image_hosts` is deliberately absent: the server enforces it.
///
/// Properties:
/// * [linkHosts] - Lower-case hosts whose `openUrl` targets open without a confirmation prompt. Empty means every link asks first.
@BuiltValue()
abstract class UiCapabilities
    implements Built<UiCapabilities, UiCapabilitiesBuilder> {
  /// Lower-case hosts whose `openUrl` targets open without a confirmation prompt. Empty means every link asks first.
  @BuiltValueField(wireName: r'link_hosts')
  BuiltList<String> get linkHosts;

  UiCapabilities._();

  factory UiCapabilities([void updates(UiCapabilitiesBuilder b)]) =
      _$UiCapabilities;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UiCapabilitiesBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UiCapabilities> get serializer =>
      _$UiCapabilitiesSerializer();
}

class _$UiCapabilitiesSerializer
    implements PrimitiveSerializer<UiCapabilities> {
  @override
  final Iterable<Type> types = const [UiCapabilities, _$UiCapabilities];

  @override
  final String wireName = r'UiCapabilities';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UiCapabilities object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'link_hosts';
    yield serializers.serialize(
      object.linkHosts,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UiCapabilities object, {
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
    required UiCapabilitiesBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'link_hosts':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.linkHosts.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UiCapabilities deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UiCapabilitiesBuilder();
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
