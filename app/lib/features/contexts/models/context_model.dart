import 'dart:convert';

import 'package:uuid/uuid.dart';

/// A named server connection profile (context).
///
/// Stores everything except [authToken] in plaintext. The token is held
/// out-of-band in [FlutterSecureStorage] by [ContextRepository].
class AssistantContext {
  const AssistantContext({
    required this.id,
    required this.name,
    required this.serverUrl,
    this.authToken,
    required this.createdAt,
  });

  final String id;

  /// Human-readable label chosen by the user.
  final String name;

  /// Base URL of the assistant server, e.g. `http://localhost:8080`.
  final String serverUrl;

  /// Bearer token.  `null` means the server requires no auth.
  /// Never serialised to JSON — stored separately in secure storage.
  final String? authToken;

  final DateTime createdAt;

  /// Creates a new context with a freshly generated UUID and current timestamp.
  factory AssistantContext.create({
    required String name,
    required String serverUrl,
    String? authToken,
  }) {
    return AssistantContext(
      id: const Uuid().v4(),
      name: name,
      serverUrl: serverUrl,
      authToken: authToken,
      createdAt: DateTime.now().toUtc(),
    );
  }

  AssistantContext copyWith({
    String? id,
    String? name,
    String? serverUrl,
    String? authToken,
    bool clearAuthToken = false,
    DateTime? createdAt,
  }) {
    return AssistantContext(
      id: id ?? this.id,
      name: name ?? this.name,
      serverUrl: serverUrl ?? this.serverUrl,
      authToken: clearAuthToken ? null : (authToken ?? this.authToken),
      createdAt: createdAt ?? this.createdAt,
    );
  }

  /// Serialises metadata only — [authToken] is intentionally excluded.
  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'serverUrl': serverUrl,
    'createdAt': createdAt.toIso8601String(),
  };

  /// Deserialises from [toJson].  [authToken] is NOT restored here.
  factory AssistantContext.fromJson(Map<String, dynamic> json) {
    return AssistantContext(
      id: json['id'] as String,
      name: json['name'] as String,
      serverUrl: json['serverUrl'] as String,
      createdAt: DateTime.parse(json['createdAt'] as String),
    );
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
