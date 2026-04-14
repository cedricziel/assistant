/// Server capability flags returned by GET /api/capabilities.
class ServerCapabilities {
  const ServerCapabilities({
    required this.voiceSend,
    required this.voiceReceive,
  });

  final bool voiceSend;
  final bool voiceReceive;

  factory ServerCapabilities.fromJson(Map<String, dynamic> json) {
    return ServerCapabilities(
      voiceSend: json['voice_send'] as bool? ?? false,
      voiceReceive: json['voice_receive'] as bool? ?? false,
    );
  }

  static const disabled = ServerCapabilities(
    voiceSend: false,
    voiceReceive: false,
  );
}
