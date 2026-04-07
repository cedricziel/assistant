import 'dart:async';
import 'dart:io';

import 'package:uuid/uuid.dart';

/// States emitted by [EmbeddedServerService] during its lifecycle.
sealed class EmbeddedServerState {
  const EmbeddedServerState();
}

class EmbeddedServerStarting extends EmbeddedServerState {
  const EmbeddedServerStarting();
}

class EmbeddedServerReady extends EmbeddedServerState {
  const EmbeddedServerReady({required this.baseUrl, required this.token});

  final String baseUrl;
  final String token;
}

class EmbeddedServerError extends EmbeddedServerState {
  const EmbeddedServerError({required this.message});

  final String message;
}

class EmbeddedServerStopped extends EmbeddedServerState {
  const EmbeddedServerStopped();
}

/// Manages the lifecycle of the bundled Rust `assistant` binary on macOS.
///
/// The binary is expected at `<bundle>/Contents/Resources/assistant` when
/// running inside a macOS `.app` bundle produced by `make build-macos-bundle`.
class EmbeddedServerService {
  Process? _process;

  // -- Static helpers --------------------------------------------------------

  /// Returns `true` when the bundled binary is present, non-empty (not a stub),
  /// and this platform supports spawning child processes (macOS only).
  static bool get isAvailable {
    if (!Platform.isMacOS) return false;
    final file = File(_binaryPath);
    return file.existsSync() && file.lengthSync() > 0;
  }

  /// Absolute path to the bundled `assistant` binary.
  ///
  /// The Flutter macOS executable lives at:
  ///   `Runner.app/Contents/MacOS/assistant_app`
  /// The Resources directory is two levels up (`Contents/Resources/`).
  static String get _binaryPath {
    final contentsDir = File(Platform.resolvedExecutable).parent.parent;
    return '${contentsDir.path}/Resources/assistant';
  }

  // -- Port selection --------------------------------------------------------

  /// Finds a free loopback port by binding to port 0, capturing the assigned
  /// port, and immediately closing the socket (TOCTOU is acceptable on loopback).
  Future<int> _findFreePort() async {
    final socket =
        await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final port = socket.port;
    await socket.close();
    return port;
  }

  // -- Lifecycle -------------------------------------------------------------

  /// Starts the embedded server and yields state transitions.
  ///
  /// Emits [EmbeddedServerStarting], then either [EmbeddedServerReady] or
  /// [EmbeddedServerError] depending on whether the health-check succeeds.
  Stream<EmbeddedServerState> start() async* {
    yield const EmbeddedServerStarting();

    final port = await _findFreePort();
    final token = const Uuid().v4();
    final binaryPath = _binaryPath;

    // Ensure execute permission (the binary may lose it after being copied).
    await Process.run('chmod', ['+x', binaryPath]);

    // Spawn the server process.
    _process = await Process.start(
      binaryPath,
      [
        'webui',
        'serve',
        '--listen',
        '127.0.0.1:$port',
        '--auth-token',
        token,
      ],
    );

    // Drain stdout/stderr to prevent the pipe buffer from blocking the child.
    _process!.stdout.drain<List<int>>();
    _process!.stderr.drain<List<int>>();

    // Poll /health with exponential backoff, capped at 5 s per attempt.
    final baseUrl = 'http://127.0.0.1:$port';
    var delay = const Duration(milliseconds: 200);
    const maxDelay = Duration(seconds: 5);
    final client = HttpClient();
    try {
      for (int attempt = 0; attempt < 10; attempt++) {
        await Future<void>.delayed(delay);
        delay = delay * 2;
        if (delay > maxDelay) delay = maxDelay;

        try {
          final request =
              await client.getUrl(Uri.parse('$baseUrl/health'));
          final response = await request.close();
          await response.drain<List<int>>();
          if (response.statusCode == 200) {
            yield EmbeddedServerReady(baseUrl: baseUrl, token: token);
            return;
          }
        } catch (_) {
          // Server not ready yet — retry.
        }
      }
    } finally {
      client.close();
    }

    // All attempts exhausted.
    _process?.kill(ProcessSignal.sigkill);
    _process = null;
    yield EmbeddedServerError(
      message: 'Embedded server failed to become ready after 10 health-check attempts.',
    );
  }

  /// Stops the embedded server with a graceful SIGTERM, falling back to
  /// SIGKILL after 3 seconds if the process has not exited.
  Future<void> stop() async {
    final process = _process;
    if (process == null) return;
    _process = null;

    process.kill(ProcessSignal.sigterm);

    try {
      await process.exitCode.timeout(const Duration(seconds: 3));
    } on TimeoutException {
      process.kill(ProcessSignal.sigkill);
      // Brief wait to allow the OS to clean up after SIGKILL.
      await process.exitCode
          .timeout(const Duration(milliseconds: 500))
          .catchError((_) => -1);
    }
  }
}
