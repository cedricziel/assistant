import 'dart:js_interop';

import 'package:pwa_install/pwa_install.dart';

// Web implementation that delegates to the pwa_install package.
//
// Call [setup] once at app startup to initialise JS-interop callbacks.
// Then poll [isInstallable] to check whether the install prompt is available,
// and call [install] to trigger the browser's native install dialog.
//
// The index.html JavaScript is responsible for calling hasPrompt() whenever
// beforeinstallprompt fires (both before and after Flutter initialises), which
// updates PWAInstall().hasPrompt so that [isInstallable] returns true.
//
// Safari (iOS/macOS) never fires beforeinstallprompt.  Use [isSafariInstallable]
// to detect the "on iOS Safari and not yet installed" case and show a custom
// instruction banner instead.

@JS('isSafariInstallable')
external bool _jsSafariInstallable();

abstract class PwaService {
  // True when the browser's install prompt has been captured and the app is
  // not already running as a PWA or TWA.
  static bool get isInstallable => PWAInstall().installPromptEnabled;

  // True when running in iOS Safari outside of standalone mode.
  // Safari never fires beforeinstallprompt, so a custom "Add to Home Screen"
  // instruction banner must be shown instead of the normal install prompt.
  static bool get isSafariInstallable {
    try {
      return _jsSafariInstallable();
    } catch (_) {
      return false;
    }
  }

  // Registers JS-interop callbacks and determines the initial launch mode.
  // Must be called once before using [isInstallable] or [install].
  static void setup() {
    PWAInstall().setup(installCallback: null);
  }

  // Triggers the browser's native "Add to Home Screen" / install dialog.
  static void install() {
    PWAInstall().promptInstall_();
  }
}
