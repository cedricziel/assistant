import 'dart:convert';
import 'dart:math';

import 'package:crypto/crypto.dart';

/// PKCE (RFC 7636) helper for OAuth2 Authorization Code flow.
///
/// Generates a cryptographically random code verifier and derives the
/// S256 code challenge from it.
class PkceChallenge {
  PkceChallenge._({required this.verifier, required this.challenge});

  /// The code verifier — a high-entropy random string sent in the token
  /// exchange request.
  final String verifier;

  /// The S256 code challenge — `BASE64URL(SHA256(verifier))`, sent in the
  /// authorization request.
  final String challenge;

  /// Generates a new PKCE challenge pair.
  ///
  /// The [length] parameter controls the number of random bytes (default 32,
  /// which produces a 43-character base64url verifier).
  factory PkceChallenge.generate({int length = 32}) {
    final verifier = _generateVerifier(length);
    final challenge = _deriveChallenge(verifier);
    return PkceChallenge._(verifier: verifier, challenge: challenge);
  }

  /// Derives the S256 challenge from a given [verifier].
  ///
  /// Useful for testing or when you need to recompute the challenge.
  static String deriveChallenge(String verifier) => _deriveChallenge(verifier);

  static String _generateVerifier(int length) {
    final random = Random.secure();
    final bytes = List<int>.generate(length, (_) => random.nextInt(256));
    return base64UrlEncode(bytes).replaceAll('=', '');
  }

  static String _deriveChallenge(String verifier) {
    final digest = sha256.convert(utf8.encode(verifier));
    return base64UrlEncode(digest.bytes).replaceAll('=', '');
  }

  @override
  String toString() =>
      'PkceChallenge(verifier: ${verifier.substring(0, 8)}...)';
}
