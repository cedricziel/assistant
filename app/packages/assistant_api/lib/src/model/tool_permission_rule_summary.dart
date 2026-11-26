//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'tool_permission_rule_summary.g.dart';

/// A saved rule.
///
/// Properties:
/// * [createdAt] - RFC 3339 creation time.
/// * [effect] - `allow` or `deny`.
/// * [id]
/// * [rule] - Rule text, for example `bash(git status*)`.
/// * [toolName] - The tool part of the rule, for example `bash`.
/// * [userId] - The user the rule belongs to.
@BuiltValue()
abstract class ToolPermissionRuleSummary
    implements
        Built<ToolPermissionRuleSummary, ToolPermissionRuleSummaryBuilder> {
  /// RFC 3339 creation time.
  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  /// `allow` or `deny`.
  @BuiltValueField(wireName: r'effect')
  String get effect;

  @BuiltValueField(wireName: r'id')
  String get id;

  /// Rule text, for example `bash(git status*)`.
  @BuiltValueField(wireName: r'rule')
  String get rule;

  /// The tool part of the rule, for example `bash`.
  @BuiltValueField(wireName: r'tool_name')
  String get toolName;

  /// The user the rule belongs to.
  @BuiltValueField(wireName: r'user_id')
  String get userId;

  ToolPermissionRuleSummary._();

  factory ToolPermissionRuleSummary(
          [void updates(ToolPermissionRuleSummaryBuilder b)]) =
      _$ToolPermissionRuleSummary;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ToolPermissionRuleSummaryBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ToolPermissionRuleSummary> get serializer =>
      _$ToolPermissionRuleSummarySerializer();
}

class _$ToolPermissionRuleSummarySerializer
    implements PrimitiveSerializer<ToolPermissionRuleSummary> {
  @override
  final Iterable<Type> types = const [
    ToolPermissionRuleSummary,
    _$ToolPermissionRuleSummary
  ];

  @override
  final String wireName = r'ToolPermissionRuleSummary';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ToolPermissionRuleSummary object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    yield r'effect';
    yield serializers.serialize(
      object.effect,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'rule';
    yield serializers.serialize(
      object.rule,
      specifiedType: const FullType(String),
    );
    yield r'tool_name';
    yield serializers.serialize(
      object.toolName,
      specifiedType: const FullType(String),
    );
    yield r'user_id';
    yield serializers.serialize(
      object.userId,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ToolPermissionRuleSummary object, {
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
    required ToolPermissionRuleSummaryBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'effect':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.effect = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'rule':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.rule = valueDes;
          break;
        case r'tool_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.toolName = valueDes;
          break;
        case r'user_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.userId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ToolPermissionRuleSummary deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ToolPermissionRuleSummaryBuilder();
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
