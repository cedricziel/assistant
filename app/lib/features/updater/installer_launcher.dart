import 'dart:io';

/// Launches the platform-appropriate installer after a successful
/// checksum verification and then exits the app.
abstract final class InstallerLauncher {
  /// Launches [path] using the install strategy matching the current platform
  /// and the artifact's [filename].
  ///
  /// Throws if the installer launch fails so the caller can surface an error
  /// rather than silently exiting.
  static Future<void> launch(String path, String filename) async {
    if (Platform.isMacOS) {
      await _launchMacOS(path, filename);
    } else if (Platform.isLinux) {
      await _launchLinux(path);
    } else if (Platform.isWindows) {
      await _launchWindows(path);
    } else {
      throw UnsupportedError('Self-update is not supported on this platform.');
    }
    exit(0);
  }

  // -- macOS ------------------------------------------------------------------

  static Future<void> _launchMacOS(String path, String filename) async {
    if (filename.endsWith('.dmg')) {
      final result = await Process.run('open', [path]);
      if (result.exitCode != 0) {
        throw Exception('Failed to open DMG (exit ${result.exitCode}): ${result.stderr}');
      }
    } else if (filename.endsWith('.zip')) {
      final dir = File(path).parent.path;
      final unzipResult = await Process.run('unzip', ['-o', path, '-d', dir]);
      if (unzipResult.exitCode != 0) {
        throw Exception('Failed to extract ZIP (exit ${unzipResult.exitCode}): ${unzipResult.stderr}');
      }
      // Find the extracted .app bundle.
      Directory? appBundle;
      for (final entry in Directory(dir).listSync()) {
        if (entry is Directory && entry.path.endsWith('.app')) {
          appBundle = entry;
          break;
        }
      }
      if (appBundle == null) {
        throw Exception('No .app bundle found after extracting ZIP.');
      }
      final openResult = await Process.run('open', [appBundle.path]);
      if (openResult.exitCode != 0) {
        throw Exception('Failed to open .app bundle (exit ${openResult.exitCode}): ${openResult.stderr}');
      }
    }
  }

  // -- Linux ------------------------------------------------------------------

  static Future<void> _launchLinux(String path) async {
    final chmodResult = await Process.run('chmod', ['+x', path]);
    if (chmodResult.exitCode != 0) {
      throw Exception('chmod +x failed (exit ${chmodResult.exitCode}): ${chmodResult.stderr}');
    }
    await Process.start(path, [], mode: ProcessStartMode.detached);
  }

  // -- Windows ----------------------------------------------------------------

  static Future<void> _launchWindows(String path) async {
    final result = await Process.run(path, []);
    if (result.exitCode != 0) {
      throw Exception('Installer exited with code ${result.exitCode}: ${result.stderr}');
    }
  }
}
