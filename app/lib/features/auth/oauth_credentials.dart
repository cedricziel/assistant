import 'dart:convert';

/// OAuth2 credentials associated with an [AssistantContext].
///
/// Stored separately in secure storage. The [accessToken] is the JWT used
/// for API requests; the [refreshToken] enables silent renewal.
class OAuthCredentials {
  const OAuthCredentials({
    required this.accessToken,
    required this.refreshToken,
    required this.expiresAt,
    required this.clientId,
  });

  final String accessToken;
  final String refreshToken;

  /// UTC instant when [accessToken] expires.
  final DateTime expiresAt;

  /// The OAuth2 client ID used to obtain these tokens.
  final String clientId;

  /// Returns `true` when the access token has expired or will expire within
  /// the given [buffer] (default 60 seconds).
  bool isExpired({Duration buffer = const Duration(seconds: 60)}) {
    return DateTime.now().toUtc().isAfter(expiresAt.subtract(buffer));
  }

  /// The token to use in `Authorization: Bearer` headers.
  /// Returns the access token regardless of expiry — callers should check
  /// [isExpired] and refresh first.
  String get bearerToken => accessToken;

  Map<String, dynamic> toJson() => {
    'accessToken': accessToken,
    'refreshToken': refreshToken,
    'expiresAt': expiresAt.toIso8601String(),
    'clientId': clientId,
  };

  factory OAuthCredentials.fromJson(Map<String, dynamic> json) {
    return OAuthCredentials(
      accessToken: json['accessToken'] as String,
      refreshToken: json['refreshToken'] as String,
      expiresAt: DateTime.parse(json['expiresAt'] as String),
      clientId: json['clientId'] as String,
    );
  }

  String toJsonString() => jsonEncode(toJson());

  factory OAuthCredentials.fromJsonString(String source) =>
      OAuthCredentials.fromJson(jsonDecode(source) as Map<String, dynamic>);

  OAuthCredentials copyWith({
    String? accessToken,
    String? refreshToken,
    DateTime? expiresAt,
    String? clientId,
  }) {
    return OAuthCredentials(
      accessToken: accessToken ?? this.accessToken,
      refreshToken: refreshToken ?? this.refreshToken,
      expiresAt: expiresAt ?? this.expiresAt,
      clientId: clientId ?? this.clientId,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is OAuthCredentials &&
          accessToken == other.accessToken &&
          refreshToken == other.refreshToken &&
          expiresAt == other.expiresAt &&
          clientId == other.clientId;

  @override
  int get hashCode =>
      Object.hash(accessToken, refreshToken, expiresAt, clientId);

  @override
  String toString() =>
      'OAuthCredentials(clientId: $clientId, expiresAt: $expiresAt)';
}
