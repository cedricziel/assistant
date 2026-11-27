//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:assistant_api/src/model/message_timing_summary.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'sse_done_event.g.dart';

/// SSE `done` event: the finished reply, the last frame of a successful turn.
///
/// Properties:
/// * [attachmentIds] - Attachments the reply produced. Omitted when there are none.
/// * [content] - The full reply text.
/// * [messageId] - The saved assistant message. Absent when the turn saved none.
/// * [role] - Always `assistant`.
/// * [timing] - How long the turn took. Absent when none was measured.
@BuiltValue()
abstract class SseDoneEvent
    implements Built<SseDoneEvent, SseDoneEventBuilder> {
  /// Attachments the reply produced. Omitted when there are none.
  @BuiltValueField(wireName: r'attachment_ids')
  BuiltList<String>? get attachmentIds;

  /// The full reply text.
  @BuiltValueField(wireName: r'content')
  String get content;

  /// The saved assistant message. Absent when the turn saved none.
  @BuiltValueField(wireName: r'message_id')
  String? get messageId;

  /// Always `assistant`.
  @BuiltValueField(wireName: r'role')
  String get role;

  /// How long the turn took. Absent when none was measured.
  @BuiltValueField(wireName: r'timing')
  MessageTimingSummary? get timing;

  SseDoneEvent._();

  factory SseDoneEvent([void updates(SseDoneEventBuilder b)]) = _$SseDoneEvent;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SseDoneEventBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SseDoneEvent> get serializer => _$SseDoneEventSerializer();
}

class _$SseDoneEventSerializer implements PrimitiveSerializer<SseDoneEvent> {
  @override
  final Iterable<Type> types = const [SseDoneEvent, _$SseDoneEvent];

  @override
  final String wireName = r'SseDoneEvent';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SseDoneEvent object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    if (object.attachmentIds != null) {
      yield r'attachment_ids';
      yield serializers.serialize(
        object.attachmentIds,
        specifiedType: const FullType(BuiltList, [FullType(String)]),
      );
    }
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    if (object.messageId != null) {
      yield r'message_id';
      yield serializers.serialize(
        object.messageId,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'role';
    yield serializers.serialize(
      object.role,
      specifiedType: const FullType(String),
    );
    if (object.timing != null) {
      yield r'timing';
      yield serializers.serialize(
        object.timing,
        specifiedType: const FullType.nullable(MessageTimingSummary),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SseDoneEvent object, {
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
    required SseDoneEventBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'attachment_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.attachmentIds.replace(valueDes);
          break;
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        case r'message_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.messageId = valueDes;
          break;
        case r'role':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.role = valueDes;
          break;
        case r'timing':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(MessageTimingSummary),
          ) as MessageTimingSummary?;
          if (valueDes == null) continue;
          result.timing.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SseDoneEvent deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SseDoneEventBuilder();
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
