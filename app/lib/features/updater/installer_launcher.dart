import 'dart:io';

/// Launches the platform-appropriate installer after a successful
/// checksum verification and then exits the app.
abstract final class InstallerLauncher {
  /// Launches [path] using the install strategy matching the current platform
  /// and the artifact's [filename].
  static Future<void> launch(String path, String filename) async {
    if (Platform.isMacOS) {
      await _launchMacOS(path, filename);
    } else if (Platform.isLinux) {
      await _launchLinux(path);
    } else if (Platform.isWindows) {
      await _launchWindows(path);
    }
    exit(0);
  }

  // -- macOS ------------------------------------------------------------------

  static Future<void> _launchMacOS(String path, String filename) async {
    if (filename.endsWith('.dmg')) {
      // Open the DMG with the default Finder handler.
      await Process.run('open', [path]);
    } else if (filename.endsWith('.zip')) {
      // Extract the .zip into the same temp directory.
      final dir = File(path).parent.path;
      await Process.run('unzip', ['-o', path, '-d', dir]);
      // Find the extracted .app bundle.
      Directory? appBundle;
      for (final entry in Directory(dir).listSync()) {
        if (entry is Directory && entry.path.endsWith('.app')) {
          appBundle = entry;
          break;
        }
      }
      if (appBundle != null) {
        await Process.run('open', [appBundle.path]);
      }
    }
  }

  // -- Linux ------------------------------------------------------------------

  static Future<void> _launchLinux(String path) async {
    await Process.run('chmod', ['+x', path]);
    await Process.start(path, [], mode: ProcessStartMode.detached);
  }

  // -- Windows ----------------------------------------------------------------

  static Future<void> _launchWindows(String path) async {
    await Process.run(path, []);
  }
}

