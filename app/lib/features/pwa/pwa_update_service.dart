// Conditional export: real JS-interop implementation on web, no-op stub
// on all other platforms (macOS, etc.).
export 'pwa_update_stub.dart'
    if (dart.library.js_interop) 'pwa_update_web.dart';
