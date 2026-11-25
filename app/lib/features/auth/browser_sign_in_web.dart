import 'package:web/web.dart' as web;

import 'browser_sign_in.dart';

BrowserSignInPlatform makeBrowserSignInPlatform() => _WebBrowserSignIn();

class _WebBrowserSignIn implements BrowserSignInPlatform {
  // sessionStorage: scoped to this tab and gone when it closes, so a stale
  // verifier never outlives the sign-in it belongs to.
  static const _key = 'assistant.pendingSignIn';

  @override
  String? get hostedServerUrl => web.window.location.origin;

  @override
  String get redirectUri =>
      '${web.window.location.origin}$browserSignInCallbackPath';

  @override
  void savePending(PendingBrowserSignIn value) {
    web.window.sessionStorage.setItem(_key, value.encode());
  }

  @override
  PendingBrowserSignIn? takePending() {
    try {
      final raw = web.window.sessionStorage.getItem(_key);
      web.window.sessionStorage.removeItem(_key);
      return raw == null ? null : PendingBrowserSignIn.decode(raw);
    } catch (_) {
      return null;
    }
  }

  @override
  Future<Uri?> open(Uri url) async {
    web.window.location.assign(url.toString());
    return null;
  }
}
