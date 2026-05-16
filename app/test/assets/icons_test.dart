import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:image/image.dart' as img;

void main() {
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
        reason:
            'index.html must declare sizes="32x32" so modern browsers select favicon-32.png',
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
        final maskable = File(
          'web/icons/Icon-maskable-$size.png',
        ).readAsBytesSync();
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
