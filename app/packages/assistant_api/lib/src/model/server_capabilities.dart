//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:assistant_api/src/model/ui_capabilities.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'server_capabilities.g.dart';

/// Server capability flags.
///
/// Properties:
/// * [ui] - Generative-UI settings of the selected space's org.
/// * [voiceReceive] - Whether the server can serve TTS audio for messages (TTS configured).
/// * [voiceSend] - Whether the server can accept voice messages (STT configured).
@BuiltValue()
abstract class ServerCapabilities
    implements Built<ServerCapabilities, ServerCapabilitiesBuilder> {
  /// Generative-UI settings of the selected space's org.
  @BuiltValueField(wireName: r'ui')
  UiCapabilities get ui;

  /// Whether the server can serve TTS audio for messages (TTS configured).
  @BuiltValueField(wireName: r'voice_receive')
  bool get voiceReceive;

  /// Whether the server can accept voice messages (STT configured).
  @BuiltValueField(wireName: r'voice_send')
  bool get voiceSend;

  ServerCapabilities._();

  factory ServerCapabilities([void updates(ServerCapabilitiesBuilder b)]) =
      _$ServerCapabilities;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ServerCapabilitiesBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ServerCapabilities> get serializer =>
      _$ServerCapabilitiesSerializer();
}

class _$ServerCapabilitiesSerializer
    implements PrimitiveSerializer<ServerCapabilities> {
  @override
  final Iterable<Type> types = const [ServerCapabilities, _$ServerCapabilities];

  @override
  final String wireName = r'ServerCapabilities';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ServerCapabilities object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'ui';
    yield serializers.serialize(
      object.ui,
      specifiedType: const FullType(UiCapabilities),
    );
    yield r'voice_receive';
    yield serializers.serialize(
      object.voiceReceive,
      specifiedType: const FullType(bool),
    );
    yield r'voice_send';
    yield serializers.serialize(
      object.voiceSend,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ServerCapabilities object, {
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
    required ServerCapabilitiesBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'ui':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(UiCapabilities),
          ) as UiCapabilities;
          result.ui.replace(valueDes);
          break;
        case r'voice_receive':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.voiceReceive = valueDes;
          break;
        case r'voice_send':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.voiceSend = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ServerCapabilities deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ServerCapabilitiesBuilder();
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
