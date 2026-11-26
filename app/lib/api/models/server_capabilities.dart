/// Server capability flags returned by GET /api/capabilities.
class ServerCapabilities {
  const ServerCapabilities({
    required this.voiceSend,
    required this.voiceReceive,
    this.linkHosts = const [],
  });

  final bool voiceSend;
  final bool voiceReceive;

  /// Lower-case hosts whose A2UI `openUrl` targets open without a prompt
  /// (`ui.link_hosts`). Empty means every link asks first.
  final List<String> linkHosts;

  factory ServerCapabilities.fromJson(Map<String, dynamic> json) {
    return ServerCapabilities(
      voiceSend: json['voice_send'] as bool? ?? false,
      voiceReceive: json['voice_receive'] as bool? ?? false,
      linkHosts: [
        for (final h
            in (json['ui'] as Map?)?['link_hosts'] as List? ?? const [])
          h as String,
      ],
    );
  }

  static const disabled = ServerCapabilities(
    voiceSend: false,
    voiceReceive: false,
  );
}
