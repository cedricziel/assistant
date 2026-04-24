//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'org_summary.g.dart';

/// Summary of an organization.
///
/// Properties:
/// * [authMode]
/// * [id]
/// * [name]
/// * [slug]
@BuiltValue()
abstract class OrgSummary implements Built<OrgSummary, OrgSummaryBuilder> {
  @BuiltValueField(wireName: r'auth_mode')
  String get authMode;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'slug')
  String get slug;

  OrgSummary._();

  factory OrgSummary([void updates(OrgSummaryBuilder b)]) = _$OrgSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(OrgSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<OrgSummary> get serializer => _$OrgSummarySerializer();
}

class _$OrgSummarySerializer implements PrimitiveSerializer<OrgSummary> {
  @override
  final Iterable<Type> types = const [OrgSummary, _$OrgSummary];

  @override
  final String wireName = r'OrgSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    OrgSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'auth_mode';
    yield serializers.serialize(
      object.authMode,
      specifiedType: const FullType(String),
    );
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
    yield r'slug';
    yield serializers.serialize(
      object.slug,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    OrgSummary object, {
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
    required OrgSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'auth_mode':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.authMode = valueDes;
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
        case r'slug':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.slug = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  OrgSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = OrgSummaryBuilder();
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
