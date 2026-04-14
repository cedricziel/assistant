import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'pwa_service.dart';

// True when running in iOS Safari outside standalone mode.
// Safari never fires beforeinstallprompt, so we show a custom instruction
// banner telling the user to tap Share → "Add to Home Screen".
final safariInstallProvider = Provider<bool>(
  (_) => PwaService.isSafariInstallable,
);

// Tracks whether the browser's PWA install prompt is currently available.
//
// Always false on non-web platforms. On web, becomes true once the browser
// fires beforeinstallprompt (i.e. the app meets PWA criteria and has not yet
// been installed).
//
// After setup(), the provider polls PwaService.isInstallable every 500 ms for
// up to 10 seconds. This handles the case where beforeinstallprompt fires
// asynchronously after Flutter initialises (typical for first-time visitors).
final pwaInstallProvider = NotifierProvider<PwaInstallNotifier, bool>(
  PwaInstallNotifier.new,
);

class PwaInstallNotifier extends Notifier<bool> {
  @override
  bool build() {
    // Registers JS-interop callbacks (no-op on non-web platforms).
    PwaService.setup();

    // Prompt may already be available if beforeinstallprompt fired before or
    // during Flutter initialisation (e.g. on a page reload).
    if (PwaService.isInstallable) return true;

    // Poll briefly after startup to detect the prompt becoming available.
    // beforeinstallprompt can fire asynchronously a few seconds after load.
    var attempts = 0;
    final timer = Timer.periodic(const Duration(milliseconds: 500), (t) {
      attempts++;
      if (PwaService.isInstallable) {
        state = true;
        t.cancel();
      } else if (attempts >= 20) {
        // Stop polling after 10 seconds.
        t.cancel();
      }
    });
    ref.onDispose(timer.cancel);

    return false;
  }

  // Trigger the native browser install dialog and mark the prompt as consumed.
  void install() {
    PwaService.install();
    state = false;
  }
}
