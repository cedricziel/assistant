import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/updater/update_service.dart';

void main() {
  final macDmg = ReleaseAsset(
    name: 'assistant-v1.0.0-macos-arm64.dmg',
    browserDownloadUrl: 'https://example.com/macos.dmg',
    size: 1000,
  );
  final macZip = ReleaseAsset(
    name: 'assistant-v1.0.0-macos.zip',
    browserDownloadUrl: 'https://example.com/macos.zip',
    size: 900,
  );
  final linux = ReleaseAsset(
    name: 'assistant-v1.0.0-x86_64.AppImage',
    browserDownloadUrl: 'https://example.com/linux.AppImage',
    size: 800,
  );
  final windows = ReleaseAsset(
    name: 'assistant-v1.0.0-windows.exe',
    browserDownloadUrl: 'https://example.com/windows.exe',
    size: 700,
  );

  group('Platform asset suffix matching', () {
    List<ReleaseAsset> all() => [macDmg, macZip, linux, windows];

    test('macOS suffix selects .dmg or -macos.zip', () {
      final macAssets = all().where(
        (a) => a.name.endsWith('.dmg') || a.name.endsWith('-macos.zip'),
      );
      expect(macAssets, isNotEmpty);
    });

    test('Linux suffix selects .AppImage', () {
      final linuxAssets = all().where((a) => a.name.endsWith('.AppImage'));
      expect(linuxAssets.length, equals(1));
      expect(linuxAssets.first, equals(linux));
    });

    test('Windows suffix selects .exe or -windows.zip', () {
      final winAssets = all().where(
        (a) => a.name.endsWith('.exe') || a.name.endsWith('-windows.zip'),
      );
      expect(winAssets.length, equals(1));
      expect(winAssets.first, equals(windows));
    });

    test('no match returns empty when no known suffix present', () {
      final assets = [
        ReleaseAsset(
          name: 'assistant-v1.0.0-unknown.tar.gz',
          browserDownloadUrl: 'https://example.com/unknown.tar.gz',
          size: 500,
        ),
      ];
      final hasMac = assets.any(
        (a) => a.name.endsWith('.dmg') || a.name.endsWith('-macos.zip'),
      );
      final hasLinux = assets.any((a) => a.name.endsWith('.AppImage'));
      final hasWindows = assets.any(
        (a) => a.name.endsWith('.exe') || a.name.endsWith('-windows.zip'),
      );
      expect(hasMac || hasLinux || hasWindows, isFalse);
    });
  });

  group('checksums.sha256 asset detection', () {
    test('locates checksums.sha256 by exact name', () {
      final checksum = ReleaseAsset(
        name: 'checksums.sha256',
        browserDownloadUrl: 'https://example.com/checksums.sha256',
        size: 128,
      );
      final assets = [macDmg, checksum, linux];
      final found = assets.where((a) => a.name == 'checksums.sha256');
      expect(found.length, equals(1));
    });

    test('returns null when checksums.sha256 is absent', () {
      final assets = [macDmg, linux];
      final found = assets.firstWhereOrNull(
        (a) => a.name == 'checksums.sha256',
      );
      expect(found, isNull);
    });
  });
}

extension _FirstWhereOrNull<T> on List<T> {
  T? firstWhereOrNull(bool Function(T) test) {
    for (final e in this) {
      if (test(e)) return e;
    }
    return null;
  }
}
