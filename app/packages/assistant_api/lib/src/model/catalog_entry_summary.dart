//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/catalog_auth_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'catalog_entry_summary.g.dart';

/// One installable remote MCP server from the integrations.sh catalog.
///
/// Properties:
/// * [auth]
/// * [categories]
/// * [connectUrl] - HTTPS endpoint of the remote MCP server.
/// * [description]
/// * [domain]
/// * [iconUrl]
/// * [id] - Upstream catalog id, e.g. `curated/linear-app-mcp`. Contains a slash.
/// * [name]
@BuiltValue()
abstract class CatalogEntrySummary
    implements Built<CatalogEntrySummary, CatalogEntrySummaryBuilder> {
  @BuiltValueField(wireName: r'auth')
  CatalogAuthResponse get auth;

  @BuiltValueField(wireName: r'categories')
  BuiltList<String> get categories;

  /// HTTPS endpoint of the remote MCP server.
  @BuiltValueField(wireName: r'connect_url')
  String get connectUrl;

  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'domain')
  String get domain;

  @BuiltValueField(wireName: r'icon_url')
  String? get iconUrl;

  /// Upstream catalog id, e.g. `curated/linear-app-mcp`. Contains a slash.
  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  CatalogEntrySummary._();

  factory CatalogEntrySummary([void updates(CatalogEntrySummaryBuilder b)]) =
      _$CatalogEntrySummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CatalogEntrySummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CatalogEntrySummary> get serializer =>
      _$CatalogEntrySummarySerializer();
}

class _$CatalogEntrySummarySerializer
    implements PrimitiveSerializer<CatalogEntrySummary> {
  @override
  final Iterable<Type> types = const [
    CatalogEntrySummary,
    _$CatalogEntrySummary
  ];

  @override
  final String wireName = r'CatalogEntrySummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CatalogEntrySummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'auth';
    yield serializers.serialize(
      object.auth,
      specifiedType: const FullType(CatalogAuthResponse),
    );
    yield r'categories';
    yield serializers.serialize(
      object.categories,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'connect_url';
    yield serializers.serialize(
      object.connectUrl,
      specifiedType: const FullType(String),
    );
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'domain';
    yield serializers.serialize(
      object.domain,
      specifiedType: const FullType(String),
    );
    if (object.iconUrl != null) {
      yield r'icon_url';
      yield serializers.serialize(
        object.iconUrl,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CatalogEntrySummary object, {
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
    required CatalogEntrySummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(CatalogAuthResponse),
          ) as CatalogAuthResponse;
          result.auth.replace(valueDes);
          break;
        case r'categories':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.categories.replace(valueDes);
          break;
        case r'connect_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.connectUrl = valueDes;
          break;
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'domain':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.domain = valueDes;
          break;
        case r'icon_url':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.iconUrl = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CatalogEntrySummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CatalogEntrySummaryBuilder();
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
