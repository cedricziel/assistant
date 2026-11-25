import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'browser_sign_in_stub.dart'
    if (dart.library.js_interop) 'browser_sign_in_web.dart'
    as platform;

/// App route the server redirects to with the authorization code.
const browserSignInCallbackPath = '/login/callback';

/// A sign-in started in this browser tab, kept until the server redirects
/// back to the app with an authorization code.
class PendingBrowserSignIn {
  const PendingBrowserSignIn({
    required this.serverUrl,
    required this.clientId,
    required this.redirectUri,
    required this.verifier,
    required this.state,
  });

  final String serverUrl;
  final String clientId;
  final String redirectUri;

  /// PKCE code verifier for the challenge sent to `/oauth/authorize`.
  final String verifier;

  /// Value the callback must echo back; guards against forged redirects.
  final String state;

  String encode() => jsonEncode({
    'serverUrl': serverUrl,
    'clientId': clientId,
    'redirectUri': redirectUri,
    'verifier': verifier,
    'state': state,
  });

  static PendingBrowserSignIn? decode(String raw) {
    try {
      final json = jsonDecode(raw) as Map<String, dynamic>;
      return PendingBrowserSignIn(
        serverUrl: json['serverUrl'] as String,
        clientId: json['clientId'] as String,
        redirectUri: json['redirectUri'] as String,
        verifier: json['verifier'] as String,
        state: json['state'] as String,
      );
    } catch (_) {
      return null;
    }
  }
}

/// Browser side of the redirect sign-in: where the app runs, how it leaves
/// for the server's sign-in page, and where the pending sign-in survives
/// the round trip.
abstract class BrowserSignInPlatform {
  /// Whether the app can sign in by redirecting this window (web only).
  bool get isSupported;

  /// Origin the app is served from, e.g. `https://assistant.example.com`.
  String get appOrigin;

  void savePending(PendingBrowserSignIn value);

  /// Returns the pending sign-in and forgets it, so a code is used once.
  PendingBrowserSignIn? takePending();

  void navigateTo(Uri url);
}

final browserSignInPlatformProvider = Provider<BrowserSignInPlatform>((ref) {
  if (kIsWeb) return platform.makeBrowserSignInPlatform();
  return _UnsupportedBrowserSignIn();
});

class _UnsupportedBrowserSignIn implements BrowserSignInPlatform {
  @override
  bool get isSupported => false;
  @override
  String get appOrigin => '';
  @override
  void savePending(PendingBrowserSignIn value) {}
  @override
  PendingBrowserSignIn? takePending() => null;
  @override
  void navigateTo(Uri url) {}
}
