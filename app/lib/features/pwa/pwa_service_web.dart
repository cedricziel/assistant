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
abstract class PwaService {
  // True when the browser's install prompt has been captured and the app is
  // not already running as a PWA or TWA.
  static bool get isInstallable => PWAInstall().installPromptEnabled;

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
