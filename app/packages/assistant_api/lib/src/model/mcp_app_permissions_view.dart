//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'mcp_app_permissions_view.g.dart';

/// The device permissions the app asks for.
///
/// Properties:
/// * [camera]
/// * [clipboardWrite]
/// * [geolocation]
/// * [microphone]
@BuiltValue()
abstract class McpAppPermissionsView
    implements Built<McpAppPermissionsView, McpAppPermissionsViewBuilder> {
  @BuiltValueField(wireName: r'camera')
  bool get camera;

  @BuiltValueField(wireName: r'clipboard_write')
  bool get clipboardWrite;

  @BuiltValueField(wireName: r'geolocation')
  bool get geolocation;

  @BuiltValueField(wireName: r'microphone')
  bool get microphone;

  McpAppPermissionsView._();

  factory McpAppPermissionsView(
      [void updates(McpAppPermissionsViewBuilder b)]) = _$McpAppPermissionsView;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(McpAppPermissionsViewBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<McpAppPermissionsView> get serializer =>
      _$McpAppPermissionsViewSerializer();
}

class _$McpAppPermissionsViewSerializer
    implements PrimitiveSerializer<McpAppPermissionsView> {
  @override
  final Iterable<Type> types = const [
    McpAppPermissionsView,
    _$McpAppPermissionsView
  ];

  @override
  final String wireName = r'McpAppPermissionsView';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    McpAppPermissionsView object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'camera';
    yield serializers.serialize(
      object.camera,
      specifiedType: const FullType(bool),
    );
    yield r'clipboard_write';
    yield serializers.serialize(
      object.clipboardWrite,
      specifiedType: const FullType(bool),
    );
    yield r'geolocation';
    yield serializers.serialize(
      object.geolocation,
      specifiedType: const FullType(bool),
    );
    yield r'microphone';
    yield serializers.serialize(
      object.microphone,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    McpAppPermissionsView object, {
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
    required McpAppPermissionsViewBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'camera':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.camera = valueDes;
          break;
        case r'clipboard_write':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.clipboardWrite = valueDes;
          break;
        case r'geolocation':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.geolocation = valueDes;
          break;
        case r'microphone':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.microphone = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  McpAppPermissionsView deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = McpAppPermissionsViewBuilder();
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
