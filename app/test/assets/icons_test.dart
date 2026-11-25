import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:image/image.dart' as img;

img.Image _decode(String path) {
  final decoded = img.decodePng(File(path).readAsBytesSync());
  expect(decoded, isNotNull, reason: '$path is not a valid PNG');
  return decoded!;
}

void main() {
  group('vector sources', () {
    for (final path in ['icon_source.svg', 'icon_glyph.svg']) {
      test('$path exists', () {
        expect(
          File(path).existsSync(),
          isTrue,
          reason: '$path is the vector master that `make icons` renders from',
        );
      });
    }
  });

  group('native app icons', () {
    test('iOS 1024 marketing icon is fully opaque', () {
      final icon = _decode(
        'ios/Runner/Assets.xcassets/AppIcon.appiconset/Icon-App-1024x1024@1x.png',
      );
      final maxAlpha = icon.maxChannelValue;
      final translucent = icon.where((p) => p.a != maxAlpha).length;
      expect(
        translucent,
        0,
        reason: 'App Store rejects iOS icons with transparency',
      );
    });

    test('macOS 1024 icon leaves transparent margin around the tile', () {
      final icon = _decode(
        'macos/Runner/Assets.xcassets/AppIcon.appiconset/app_icon_1024.png',
      );
      for (final (x, y) in [
        (0, 0),
        (1023, 0),
        (0, 1023),
        (1023, 1023),
        (512, 40),
      ]) {
        expect(
          icon.getPixel(x, y).a,
          0,
          reason:
              'macOS icons sit on an 824px tile with transparent margin; '
              'pixel ($x,$y) must be transparent',
        );
      }
      expect(
        icon.getPixel(512, 512).a,
        icon.maxChannelValue,
        reason: 'the tile itself must be opaque',
      );
    });
  });

  group('iOS launch image', () {
    const dir = 'ios/Runner/Assets.xcassets/LaunchImage.imageset';
    for (final (name, size) in [
      ('LaunchImage.png', 200),
      ('LaunchImage@2x.png', 400),
      ('LaunchImage@3x.png', 600),
    ]) {
      test('$name shows the current brand artwork', () {
        final launch = _decode('$dir/$name');
        expect(launch.width, size);
        expect(launch.height, size);

        const probe = 32;
        final a = img.copyResize(launch, width: probe, height: probe);
        final b = img.copyResize(
          _decode('icon_source.png'),
          width: probe,
          height: probe,
        );
        var diff = 0.0;
        for (var y = 0; y < probe; y++) {
          for (var x = 0; x < probe; x++) {
            final p = a.getPixel(x, y);
            final q = b.getPixel(x, y);
            diff += (p.r - q.r).abs() + (p.g - q.g).abs() + (p.b - q.b).abs();
          }
        }
        final meanDiff = diff / (probe * probe * 3);
        expect(
          meanDiff,
          lessThan(4),
          reason:
              'launch image drifted from icon_source.png — run `make icons`',
        );
      });
    }
  });

  group('web/icons/badge-96.png', () {
    test('is a 96×96 white-on-transparent notification badge', () {
      final badge = _decode('web/icons/badge-96.png');
      expect(badge.width, 96);
      expect(badge.height, 96);
      final max = badge.maxChannelValue;
      final visible = badge.where((p) => p.a > 0).toList();
      expect(visible, isNotEmpty, reason: 'badge must draw the monogram');
      expect(
        visible.where((p) => p.r != max || p.g != max || p.b != max),
        isEmpty,
        reason: 'Android draws badges from alpha only; keep them pure white',
      );
      expect(
        badge.getPixel(0, 0).a,
        0,
        reason: 'badge background must be transparent',
      );
    });

    test('sw.js uses it as the notification badge', () {
      expect(
        File('web/sw.js').readAsStringSync(),
        contains('badge: "/icons/badge-96.png"'),
      );
    });
  });

  group('assets/tray_icon.png', () {
    test('is a monochrome template image', () {
      final icon = _decode('assets/tray_icon.png');
      final coloured = icon
          .where((p) => p.a > 0 && (p.r != 0 || p.g != 0 || p.b != 0))
          .length;
      expect(
        coloured,
        0,
        reason:
            'macOS template images must be black with alpha so the menu '
            'bar can tint them for light and dark mode',
      );
    });
  });

  group('web/favicon.png (legacy 16×16)', () {
    final favicon = File('web/favicon.png');

    test('exists', () {
      expect(
        favicon.existsSync(),
        isTrue,
        reason: 'web/favicon.png must be checked in',
      );
    });

    test('is exactly 16×16', () {
      final decoded = img.decodePng(favicon.readAsBytesSync());
      expect(decoded, isNotNull);
      expect(decoded!.width, equals(16));
      expect(decoded.height, equals(16));
    });
  });

  group('web/favicon-32.png (modern tab-icon size)', () {
    final favicon32 = File('web/favicon-32.png');

    test('exists', () {
      expect(
        favicon32.existsSync(),
        isTrue,
        reason:
            'web/favicon-32.png must be checked in — generate via `make icons`',
      );
    });

    test('is exactly 32×32', () {
      final decoded = img.decodePng(favicon32.readAsBytesSync());
      expect(decoded, isNotNull, reason: 'favicon-32.png is not a valid PNG');
      expect(decoded!.width, equals(32));
      expect(decoded.height, equals(32));
    });
  });

  group('web/index.html', () {
    test('references favicon-32.png with sizes="32x32"', () {
      final html = File('web/index.html').readAsStringSync();
      expect(
        html,
        contains('favicon-32.png'),
        reason: 'index.html must reference favicon-32.png',
      );
      expect(
        html,
        contains('sizes="32x32"'),
        reason: 'index.html must declare sizes="32x32" so modern browsers select favicon-32.png',
      );
    });
  });

  group('maskable PWA icons', () {
    for (final size in [192, 512]) {
      group('Icon-maskable-$size.png', () {
        final file = File('web/icons/Icon-maskable-$size.png');

        test('exists', () {
          expect(
            file.existsSync(),
            isTrue,
            reason: 'web/icons/Icon-maskable-$size.png must be checked in',
          );
        });

        test('has fully opaque outer 10% border', () {
          final decoded = img.decodePng(file.readAsBytesSync());
          expect(
            decoded,
            isNotNull,
            reason: 'Icon-maskable-$size.png is not a valid PNG',
          );
          expect(decoded!.width, equals(size));
          expect(decoded.height, equals(size));

          final border = (size * 0.10).round();
          final maxAlpha = decoded.maxChannelValue.toDouble();
          final transparentPixels = <String>[];

          for (var y = 0; y < size; y++) {
            for (var x = 0; x < size; x++) {
              final inBorder =
                  x < border ||
                  x >= size - border ||
                  y < border ||
                  y >= size - border;
              if (!inBorder) continue;
              final p = decoded.getPixel(x, y);
              if (p.a.toDouble() != maxAlpha) {
                transparentPixels.add('($x,$y)=a${p.a}/max$maxAlpha');
                if (transparentPixels.length >= 5) {
                  break;
                }
              }
            }
            if (transparentPixels.length >= 5) break;
          }

          expect(
            transparentPixels,
            isEmpty,
            reason:
                'maskable Icon-$size.png must have a fully opaque outer 10% border so '
                'Android mask shapes do not expose transparency. '
                'First offending pixels: ${transparentPixels.join(', ')}',
          );
        });
      });
    }

    test('maskable variants differ from their standard counterparts', () {
      for (final size in [192, 512]) {
        final standard = File('web/icons/Icon-$size.png').readAsBytesSync();
        final maskable = File('web/icons/Icon-maskable-$size.png')
            .readAsBytesSync();
        final stdHash = sha256.convert(standard).toString();
        final mskHash = sha256.convert(maskable).toString();
        expect(
          mskHash,
          isNot(equals(stdHash)),
          reason:
              'Icon-maskable-$size.png is byte-identical to Icon-$size.png — '
              'maskable variant needs different (safe-zone) source artwork',
        );
      }
    });
  });
}
