import 'dart:convert';

/// A saved server connection profile.
///
/// Stored in [flutter_secure_storage] — never sent to the server.
class ServerProfile {
  const ServerProfile({
    required this.baseUrl,
    required this.token,
    this.label = '',
  });

  /// Base URL of the assistant server, e.g. `http://localhost:8080`.
  final String baseUrl;

  /// Bearer authentication token (stored encrypted at rest).
  final String token;

  /// Human-readable display label, defaults to the hostname of [baseUrl].
  final String label;

  /// Display label falling back to the hostname if [label] is empty.
  String get displayLabel {
    if (label.isNotEmpty) return label;
    try {
      return Uri.parse(baseUrl).host;
    } catch (_) {
      return baseUrl;
    }
  }

  // -- Serialisation for flutter_secure_storage -------------------------------

  String toJson() =>
      jsonEncode({'baseUrl': baseUrl, 'token': token, 'label': label});

  factory ServerProfile.fromJson(String source) {
    final map = jsonDecode(source) as Map<String, dynamic>;
    return ServerProfile(
      baseUrl: map['baseUrl'] as String,
      token: map['token'] as String,
      label: map['label'] as String? ?? '',
    );
  }

  ServerProfile copyWith({String? baseUrl, String? token, String? label}) {
    return ServerProfile(
      baseUrl: baseUrl ?? this.baseUrl,
      token: token ?? this.token,
      label: label ?? this.label,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ServerProfile &&
          baseUrl == other.baseUrl &&
          token == other.token &&
          label == other.label;

  @override
  int get hashCode => Object.hash(baseUrl, token, label);
}
