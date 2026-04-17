import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:pub_semver/pub_semver.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../core/constants.dart';

const _kLastCheckKey = 'updater.last_check_date';

/// Metadata for a single release asset.
class ReleaseAsset {
  const ReleaseAsset({
    required this.name,
    required this.browserDownloadUrl,
    required this.size,
  });

  final String name;
  final String browserDownloadUrl;
  final int size;

  factory ReleaseAsset.fromJson(Map<String, dynamic> json) => ReleaseAsset(
    name: json['name'] as String,
    browserDownloadUrl: json['browser_download_url'] as String,
    size: json['size'] as int,
  );
}

/// Metadata returned from the GitHub Releases API.
class ReleaseInfo {
  const ReleaseInfo({
    required this.tagName,
    required this.body,
    required this.htmlUrl,
    required this.assets,
  });

  final String tagName;
  final String body;
  final String htmlUrl;
  final List<ReleaseAsset> assets;

  factory ReleaseInfo.fromJson(Map<String, dynamic> json) => ReleaseInfo(
    tagName: json['tag_name'] as String,
    body: (json['body'] as String?) ?? '',
    htmlUrl: json['html_url'] as String,
    assets: (json['assets'] as List<dynamic>)
        .map((a) => ReleaseAsset.fromJson(a as Map<String, dynamic>))
        .toList(),
  );
}

/// Holds the information surfaced to the UI when an update is available.
class UpdateInfo {
  const UpdateInfo({
    required this.currentVersion,
    required this.latestVersion,
    required this.release,
  });

  final String currentVersion;
  final String latestVersion;
  final ReleaseInfo release;

  /// One-line release notes (first non-empty line of the body).
  String get shortReleaseNotes {
    final lines = release.body.split('\n');
    for (final line in lines) {
      final trimmed = line.trim();
      if (trimmed.isNotEmpty) return trimmed;
    }
    return 'New version available';
  }
}

/// Service that wraps GitHub Releases API checks.
///
/// Handles:
/// - Fetching the latest release metadata
/// - Version comparison via [pub_semver]
/// - Rate-limiting to at most one check per calendar day
class UpdateService {
  UpdateService({Dio? dio}) : _dio = dio ?? Dio();

  final Dio _dio;

  /// Fetch the latest release from GitHub.  Returns [null] on any error.
  Future<ReleaseInfo?> fetchLatestRelease() async {
    try {
      final response = await _dio.get<Map<String, dynamic>>(
        Uri.https(
          'api.github.com',
          '/repos/$kGitHubOwner/$kGitHubRepo/releases/latest',
        ).toString(),
        options: Options(
          headers: {'Accept': 'application/vnd.github+json'},
          receiveTimeout: const Duration(seconds: 10),
          sendTimeout: const Duration(seconds: 10),
        ),
      );
      if (response.statusCode == 200 && response.data != null) {
        return ReleaseInfo.fromJson(response.data!);
      }
    } catch (_) {
      // Fail silently — network errors must not surface to the user.
    }
    return null;
  }

  /// Returns [true] when [latest] is strictly greater than [current].
  ///
  /// Both strings are expected to be SemVer (optionally prefixed with 'v').
  bool isNewerVersion(String latest, String current) {
    try {
      final l = Version.parse(
        latest.startsWith('v') ? latest.substring(1) : latest,
      );
      final c = Version.parse(
        current.startsWith('v') ? current.substring(1) : current,
      );
      return l > c;
    } catch (_) {
      return false;
    }
  }

  /// Checks whether a full update check should be skipped today.
  Future<bool> _alreadyCheckedToday() async {
    final prefs = await SharedPreferences.getInstance();
    final stored = prefs.getString(_kLastCheckKey);
    if (stored == null) return false;
    final today = DateTime.now().toIso8601String().substring(0, 10);
    return stored == today;
  }

  Future<void> _recordCheckDate() async {
    final prefs = await SharedPreferences.getInstance();
    final today = DateTime.now().toIso8601String().substring(0, 10);
    await prefs.setString(_kLastCheckKey, today);
  }

  /// Entry-point for the update-check flow.
  ///
  /// Returns [UpdateInfo] when a newer version is available, [null] otherwise.
  /// Always returns [null] in debug/profile builds or on web.
  Future<UpdateInfo?> checkForUpdate() async {
    if (kIsWeb || !kReleaseMode) return null;
    // Self-update is not supported on mobile platforms (iOS, Android)
    if (Platform.isIOS || Platform.isAndroid) return null;
    if (!Platform.isMacOS && !Platform.isLinux && !Platform.isWindows) {
      return null;
    }
    if (await _alreadyCheckedToday()) return null;

    final release = await fetchLatestRelease();

    // Only record the check date when a valid release was fetched.
    // A network failure should not suppress tomorrow's check.
    if (release == null) return null;
    await _recordCheckDate();

    final info = await PackageInfo.fromPlatform();
    final current = info.version;

    if (!isNewerVersion(release.tagName, current)) return null;

    return UpdateInfo(
      currentVersion: current,
      latestVersion: release.tagName,
      release: release,
    );
  }
}
