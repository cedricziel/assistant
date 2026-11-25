//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/catalog_entry_summary.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'catalog_entry_list_response.g.dart';

/// One page of catalog search results.
///
/// Properties:
/// * [items]
/// * [nextCursor] - Pass as `cursor` to fetch the next page; absent on the last page.
@BuiltValue()
abstract class CatalogEntryListResponse
    implements
        Built<CatalogEntryListResponse, CatalogEntryListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<CatalogEntrySummary> get items;

  /// Pass as `cursor` to fetch the next page; absent on the last page.
  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  CatalogEntryListResponse._();

  factory CatalogEntryListResponse(
          [void updates(CatalogEntryListResponseBuilder b)]) =
      _$CatalogEntryListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CatalogEntryListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CatalogEntryListResponse> get serializer =>
      _$CatalogEntryListResponseSerializer();
}

class _$CatalogEntryListResponseSerializer
    implements PrimitiveSerializer<CatalogEntryListResponse> {
  @override
  final Iterable<Type> types = const [
    CatalogEntryListResponse,
    _$CatalogEntryListResponse
  ];

  @override
  final String wireName = r'CatalogEntryListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CatalogEntryListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(CatalogEntrySummary)]),
    );
    if (object.nextCursor != null) {
      yield r'next_cursor';
      yield serializers.serialize(
        object.nextCursor,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    CatalogEntryListResponse object, {
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
    required CatalogEntryListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType:
                const FullType(BuiltList, [FullType(CatalogEntrySummary)]),
          ) as BuiltList<CatalogEntrySummary>;
          result.items.replace(valueDes);
          break;
        case r'next_cursor':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nextCursor = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CatalogEntryListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CatalogEntryListResponseBuilder();
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
