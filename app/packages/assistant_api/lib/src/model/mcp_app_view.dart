//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/mcp_app_permissions_view.dart';
import 'package:assistant_api/src/model/mcp_app_csp_view.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'mcp_app_view.g.dart';

/// The interactive UI an external MCP tool declared for its result.
///
/// Properties:
/// * [csp]
/// * [html] - The app document, served as `text/html;profile=mcp-app`.
/// * [permissions]
/// * [prefersBorder] - Whether the app asks to be drawn with a border.
/// * [resourceUri] - The `ui://` resource the HTML was read from.
/// * [server] - Namespace of the server the tool came from, as in `mcp__{server}__{tool}`.
@BuiltValue()
abstract class McpAppView implements Built<McpAppView, McpAppViewBuilder> {
  @BuiltValueField(wireName: r'csp')
  McpAppCspView get csp;

  /// The app document, served as `text/html;profile=mcp-app`.
  @BuiltValueField(wireName: r'html')
  String get html;

  @BuiltValueField(wireName: r'permissions')
  McpAppPermissionsView get permissions;

  /// Whether the app asks to be drawn with a border.
  @BuiltValueField(wireName: r'prefers_border')
  bool? get prefersBorder;

  /// The `ui://` resource the HTML was read from.
  @BuiltValueField(wireName: r'resource_uri')
  String get resourceUri;

  /// Namespace of the server the tool came from, as in `mcp__{server}__{tool}`.
  @BuiltValueField(wireName: r'server')
  String get server;

  McpAppView._();

  factory McpAppView([void updates(McpAppViewBuilder b)]) = _$McpAppView;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(McpAppViewBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<McpAppView> get serializer => _$McpAppViewSerializer();
}

class _$McpAppViewSerializer implements PrimitiveSerializer<McpAppView> {
  @override
  final Iterable<Type> types = const [McpAppView, _$McpAppView];

  @override
  final String wireName = r'McpAppView';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    McpAppView object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'csp';
    yield serializers.serialize(
      object.csp,
      specifiedType: const FullType(McpAppCspView),
    );
    yield r'html';
    yield serializers.serialize(
      object.html,
      specifiedType: const FullType(String),
    );
    yield r'permissions';
    yield serializers.serialize(
      object.permissions,
      specifiedType: const FullType(McpAppPermissionsView),
    );
    if (object.prefersBorder != null) {
      yield r'prefers_border';
      yield serializers.serialize(
        object.prefersBorder,
        specifiedType: const FullType.nullable(bool),
      );
    }
    yield r'resource_uri';
    yield serializers.serialize(
      object.resourceUri,
      specifiedType: const FullType(String),
    );
    yield r'server';
    yield serializers.serialize(
      object.server,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    McpAppView object, {
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
    required McpAppViewBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'csp':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(McpAppCspView),
          ) as McpAppCspView;
          result.csp.replace(valueDes);
          break;
        case r'html':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.html = valueDes;
          break;
        case r'permissions':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(McpAppPermissionsView),
          ) as McpAppPermissionsView;
          result.permissions.replace(valueDes);
          break;
        case r'prefers_border':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(bool),
          ) as bool?;
          if (valueDes == null) continue;
          result.prefersBorder = valueDes;
          break;
        case r'resource_uri':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.resourceUri = valueDes;
          break;
        case r'server':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.server = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  McpAppView deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = McpAppViewBuilder();
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
