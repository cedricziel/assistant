import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_web_auth_2/flutter_web_auth_2.dart';

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

/// Thrown by [BrowserSignInPlatform.open] when the user closes the sign-in
/// sheet without finishing.
class BrowserSignInCancelled implements Exception {
  const BrowserSignInCancelled();
}

/// Where the server's sign-in page opens and how its redirect comes back.
///
/// Web: the app's own window leaves for the server and returns to
/// [browserSignInCallbackPath], so the pending sign-in waits in storage.
/// Native: the system sign-in sheet (`ASWebAuthenticationSession`) opens and
/// hands the redirect back to [open] directly.
abstract class BrowserSignInPlatform {
  /// Server the app is served from (web), or null when the user enters one.
  String? get hostedServerUrl;

  /// Redirect URI registered for this app.
  String get redirectUri;

  void savePending(PendingBrowserSignIn value);

  /// Returns the pending sign-in and forgets it, so a code is used once.
  PendingBrowserSignIn? takePending();

  /// Opens the sign-in page at [url]. Returns the redirect the server sent
  /// back, or null when the page left the app and the redirect arrives at
  /// [browserSignInCallbackPath] instead. Throws [BrowserSignInCancelled].
  Future<Uri?> open(Uri url);
}

final browserSignInPlatformProvider = Provider<BrowserSignInPlatform>((ref) {
  if (kIsWeb) return platform.makeBrowserSignInPlatform();
  return NativeBrowserSignIn();
});

/// Private-use URI scheme for the native redirect (RFC 8252 §7.1).
const nativeSignInScheme = 'com.cedricziel.assistant';

class NativeBrowserSignIn implements BrowserSignInPlatform {
  PendingBrowserSignIn? _pending;

  @override
  String? get hostedServerUrl => null;

  @override
  String get redirectUri => '$nativeSignInScheme:/oauth/callback';

  @override
  void savePending(PendingBrowserSignIn value) => _pending = value;

  @override
  PendingBrowserSignIn? takePending() {
    final value = _pending;
    _pending = null;
    return value;
  }

  @override
  Future<Uri?> open(Uri url) async {
    try {
      final result = await FlutterWebAuth2.authenticate(
        url: url.toString(),
        callbackUrlScheme: nativeSignInScheme,
      );
      return Uri.parse(result);
    } on PlatformException catch (e) {
      if (e.code == 'CANCELED') throw const BrowserSignInCancelled();
      rethrow;
    }
  }
}
