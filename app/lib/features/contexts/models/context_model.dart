import 'dart:convert';

import 'package:uuid/uuid.dart';

import '../../auth/oauth_credentials.dart';

/// Authentication mode for a context.
enum AuthMode {
  /// Legacy single-token auth (shared token, no refresh).
  legacyToken,

  /// Full OAuth2 with access + refresh tokens.
  oauth2,
}

/// A named server connection profile (context).
///
/// Stores everything except secrets in plaintext. The [authToken] and
/// [oauthCredentials] are held out-of-band in secure storage by
/// [ContextRepository].
class AssistantContext {
  const AssistantContext({
    required this.id,
    required this.name,
    required this.serverUrl,
    this.authToken,
    this.oauthCredentials,
    this.authMode = AuthMode.legacyToken,
    required this.createdAt,
    this.credentialsCorrupted = false,
  });

  final String id;

  /// Human-readable label chosen by the user.
  final String name;

  /// Base URL of the assistant server, e.g. `http://localhost:8080`.
  final String serverUrl;

  /// Legacy bearer token.  `null` means the server requires no auth.
  /// Never serialised to JSON — stored separately in secure storage.
  final String? authToken;

  /// OAuth2 credentials (access + refresh tokens).
  /// Never serialised to JSON — stored separately in secure storage.
  final OAuthCredentials? oauthCredentials;

  /// Which auth mode this context uses.
  final AuthMode authMode;

  final DateTime createdAt;

  /// In-memory flag set by [ContextRepository.loadContexts] when secure-storage
  /// reads for this context's credentials throw — typically because
  /// `flutter_secure_storage`'s IndexedDB key has been wiped on web.
  /// Never serialised; surface for the router redirect + login banner only.
  final bool credentialsCorrupted;

  /// The bearer token to use for API requests — picks the right one based
  /// on [authMode].
  String? get effectiveToken {
    if (authMode == AuthMode.oauth2 && oauthCredentials != null) {
      return oauthCredentials!.bearerToken;
    }
    return authToken;
  }

  /// Whether the OAuth2 access token needs refreshing.
  bool get needsTokenRefresh =>
      authMode == AuthMode.oauth2 &&
      oauthCredentials != null &&
      oauthCredentials!.isExpired();

  /// Creates a new context with a freshly generated UUID and current timestamp.
  factory AssistantContext.create({
    required String name,
    required String serverUrl,
    String? authToken,
    OAuthCredentials? oauthCredentials,
    AuthMode authMode = AuthMode.legacyToken,
  }) {
    return AssistantContext(
      id: const Uuid().v4(),
      name: name,
      serverUrl: serverUrl,
      authToken: authToken,
      oauthCredentials: oauthCredentials,
      authMode: authMode,
      createdAt: DateTime.now().toUtc(),
    );
  }

  AssistantContext copyWith({
    String? id,
    String? name,
    String? serverUrl,
    String? authToken,
    bool clearAuthToken = false,
    OAuthCredentials? oauthCredentials,
    bool clearOAuthCredentials = false,
    AuthMode? authMode,
    DateTime? createdAt,
    bool? credentialsCorrupted,
  }) {
    return AssistantContext(
      id: id ?? this.id,
      name: name ?? this.name,
      serverUrl: serverUrl ?? this.serverUrl,
      authToken: clearAuthToken ? null : (authToken ?? this.authToken),
      oauthCredentials: clearOAuthCredentials
          ? null
          : (oauthCredentials ?? this.oauthCredentials),
      authMode: authMode ?? this.authMode,
      createdAt: createdAt ?? this.createdAt,
      credentialsCorrupted: credentialsCorrupted ?? this.credentialsCorrupted,
    );
  }

  /// Serialises metadata only — secrets ([authToken], [oauthCredentials])
  /// are intentionally excluded.
  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'serverUrl': serverUrl,
    'authMode': authMode.name,
    'createdAt': createdAt.toIso8601String(),
  };

  /// Deserialises from [toJson].  Secrets are NOT restored here.
  factory AssistantContext.fromJson(Map<String, dynamic> json) {
    return AssistantContext(
      id: json['id'] as String,
      name: json['name'] as String,
      serverUrl: json['serverUrl'] as String,
      authMode: _parseAuthMode(json['authMode'] as String?),
      createdAt: DateTime.parse(json['createdAt'] as String),
    );
  }

  static AuthMode _parseAuthMode(String? value) {
    if (value == AuthMode.oauth2.name) return AuthMode.oauth2;
    return AuthMode.legacyToken;
  }

  /// Convenience: encode to a JSON string.
  String toJsonString() => jsonEncode(toJson());

  /// Convenience: decode from a JSON string.
  factory AssistantContext.fromJsonString(String source) =>
      AssistantContext.fromJson(jsonDecode(source) as Map<String, dynamic>);

  @override
  bool operator ==(Object other) =>
      identical(this, other) || other is AssistantContext && id == other.id;

  @override
  int get hashCode => id.hashCode;

  @override
  String toString() =>
      'AssistantContext(id: $id, name: $name, serverUrl: $serverUrl)';
}
