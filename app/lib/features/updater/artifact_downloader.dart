import 'dart:io';

import 'package:collection/collection.dart';
import 'package:crypto/crypto.dart';
import 'package:dio/dio.dart';
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';

import 'installer_launcher.dart';
import 'update_service.dart';

/// Selects the platform-correct release asset, downloads it, verifies the
/// SHA-256 checksum, and delegates to [InstallerLauncher].
class ArtifactDownloader {
  ArtifactDownloader({Dio? dio}) : _dio = dio ?? Dio();

  final Dio _dio;

  Future<void> downloadAndInstall(
    ReleaseInfo release, {
    required void Function(int received, int total) onProgress,
    required void Function(String status) onStatus,
    required CancelToken cancelToken,
  }) async {
    // 1. Resolve platform asset.
    final asset = _selectAsset(release.assets);
    if (asset == null) {
      onStatus('No matching asset — opening release page…');
      await _openBrowser(release.htmlUrl);
      return;
    }

    // 2. Fetch checksums file (optional — fall back to browser if absent).
    final checksumMap = await _fetchChecksums(release.assets);

    // Write to the app-support directory (user-private, not world-writable).
    final supportDir = await getApplicationSupportDirectory();
    final destPath = '${supportDir.path}/${asset.name}';

    // Enforce HTTPS on the download URL.
    final downloadUri = Uri.parse(asset.browserDownloadUrl);
    if (downloadUri.scheme != 'https') {
      throw Exception(
        'Refusing to download over non-HTTPS URL: ${downloadUri.scheme}',
      );
    }

    // 3. Download the artifact.
    onStatus('Downloading ${asset.name}…');
    await _dio.download(
      asset.browserDownloadUrl,
      destPath,
      onReceiveProgress: onProgress,
      cancelToken: cancelToken,
    );

    // 4. Verify checksum (or fall back to browser when no manifest).
    if (checksumMap == null) {
      onStatus('No checksum file — opening release page…');
      await File(destPath).delete();
      await _openBrowser(release.htmlUrl);
      return;
    }

    onStatus('Verifying checksum…');
    final ok = await _verifyChecksum(destPath, checksumMap, asset.name);
    if (!ok) {
      await File(destPath).delete();
      throw Exception('Checksum mismatch — downloaded file is corrupt.');
    }

    // 5. Launch installer.
    onStatus('Launching installer…');
    await InstallerLauncher.launch(destPath, asset.name);
  }

  // -- Helpers ----------------------------------------------------------------

  /// Returns the asset matching the current platform, or [null].
  ReleaseAsset? _selectAsset(List<ReleaseAsset> assets) {
    if (Platform.isMacOS) {
      return assets.firstWhereOrNull(
        (a) => a.name.endsWith('.dmg') || a.name.endsWith('-macos.zip'),
      );
    }
    if (Platform.isLinux) {
      return assets.firstWhereOrNull((a) => a.name.endsWith('.AppImage'));
    }
    if (Platform.isWindows) {
      return assets.firstWhereOrNull(
        (a) => a.name.endsWith('.exe') || a.name.endsWith('-windows.zip'),
      );
    }
    return null;
  }

  /// Downloads and parses the `checksums.sha256` release asset.
  ///
  /// Returns a map of `filename → sha256hex`, or [null] when absent.
  Future<Map<String, String>?> _fetchChecksums(
    List<ReleaseAsset> assets,
  ) async {
    final asset = assets.firstWhereOrNull((a) => a.name == 'checksums.sha256');
    if (asset == null) return null;
    try {
      final checksumUri = Uri.parse(asset.browserDownloadUrl);
      if (checksumUri.scheme != 'https') return null;
      final response = await _dio.get<String>(
        asset.browserDownloadUrl,
        options: Options(responseType: ResponseType.plain),
      );
      final body = response.data ?? '';
      final result = <String, String>{};
      for (final line in body.split('\n')) {
        final parts = line.trim().split(RegExp(r'\s+'));
        if (parts.length >= 2) {
          result[parts[1]] = parts[0];
        }
      }
      return result;
    } catch (_) {
      return null;
    }
  }

  /// Computes the SHA-256 of [path] in a streaming fashion to avoid
  /// loading the entire artifact into memory.
  Future<bool> _verifyChecksum(
    String path,
    Map<String, String> checksumMap,
    String filename,
  ) async {
    final expected = checksumMap[filename];
    if (expected == null) return false;
    final digest = await sha256.bind(File(path).openRead()).first;
    return digest.toString() == expected;
  }

  Future<void> _openBrowser(String url) async {
    final uri = Uri.parse(url);
    if (await canLaunchUrl(uri)) {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    }
  }
}
