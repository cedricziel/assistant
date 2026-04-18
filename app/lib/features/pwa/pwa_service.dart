// Conditional export: uses the real pwa_install implementation on web and a
// no-op stub on all other platforms (macOS, etc.).
export 'pwa_service_stub.dart' if (dart.library.html) 'pwa_service_web.dart';
