//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'mcp_app_csp_view.g.dart';

/// The origins the app may reach. An empty list means the restrictive default.
///
/// Properties:
/// * [baseUriDomains]
/// * [connectDomains]
/// * [frameDomains]
/// * [resourceDomains]
@BuiltValue()
abstract class McpAppCspView
    implements Built<McpAppCspView, McpAppCspViewBuilder> {
  @BuiltValueField(wireName: r'base_uri_domains')
  BuiltList<String>? get baseUriDomains;

  @BuiltValueField(wireName: r'connect_domains')
  BuiltList<String>? get connectDomains;

  @BuiltValueField(wireName: r'frame_domains')
  BuiltList<String>? get frameDomains;

  @BuiltValueField(wireName: r'resource_domains')
  BuiltList<String>? get resourceDomains;

  McpAppCspView._();

  factory McpAppCspView([void updates(McpAppCspViewBuilder b)]) =
      _$McpAppCspView;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(McpAppCspViewBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<McpAppCspView> get serializer =>
      _$McpAppCspViewSerializer();
}

class _$McpAppCspViewSerializer implements PrimitiveSerializer<McpAppCspView> {
  @override
  final Iterable<Type> types = const [McpAppCspView, _$McpAppCspView];

  @override
  final String wireName = r'McpAppCspView';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    McpAppCspView object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.baseUriDomains != null) {
      yield r'base_uri_domains';
      yield serializers.serialize(
        object.baseUriDomains,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.connectDomains != null) {
      yield r'connect_domains';
      yield serializers.serialize(
        object.connectDomains,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.frameDomains != null) {
      yield r'frame_domains';
      yield serializers.serialize(
        object.frameDomains,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    if (object.resourceDomains != null) {
      yield r'resource_domains';
      yield serializers.serialize(
        object.resourceDomains,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    McpAppCspView object, {
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
    required McpAppCspViewBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'base_uri_domains':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.baseUriDomains.replace(valueDes);
          break;
        case r'connect_domains':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.connectDomains.replace(valueDes);
          break;
        case r'frame_domains':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.frameDomains.replace(valueDes);
          break;
        case r'resource_domains':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.resourceDomains.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  McpAppCspView deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = McpAppCspViewBuilder();
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
