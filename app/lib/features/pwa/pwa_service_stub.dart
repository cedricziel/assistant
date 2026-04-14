// No-op stub for non-web platforms (macOS, etc.).
// The PWA install feature is only meaningful in a browser context.
abstract class PwaService {
  // Always false outside of a web browser.
  static bool get isInstallable => false;

  // Always false outside of a web browser.
  static bool get isSafariInstallable => false;

  // No-op on non-web platforms.
  static void setup() {}

  // No-op on non-web platforms.
  static void install() {}
}
