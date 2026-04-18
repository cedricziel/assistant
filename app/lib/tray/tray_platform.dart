// Conditionally exports the real tray service on desktop (dart:io available)
// and the no-op stub on web (dart:io not available).
export 'tray_service_stub.dart' if (dart.library.io) 'tray_service.dart';
