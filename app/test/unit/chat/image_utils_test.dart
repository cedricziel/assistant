import 'package:assistant_app/features/chat/image_utils.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('mimeFromExtension', () {
    test('maps png', () {
      expect(mimeFromExtension('png'), 'image/png');
    });

    test('maps jpg and jpeg', () {
      expect(mimeFromExtension('jpg'), 'image/jpeg');
      expect(mimeFromExtension('jpeg'), 'image/jpeg');
    });

    test('maps gif', () {
      expect(mimeFromExtension('gif'), 'image/gif');
    });

    test('maps webp', () {
      expect(mimeFromExtension('webp'), 'image/webp');
    });

    test('maps heic and heif to image/heic', () {
      expect(mimeFromExtension('heic'), 'image/heic');
      expect(mimeFromExtension('heif'), 'image/heic');
    });

    test('is case insensitive', () {
      expect(mimeFromExtension('HEIC'), 'image/heic');
      expect(mimeFromExtension('PNG'), 'image/png');
      expect(mimeFromExtension('Jpg'), 'image/jpeg');
    });

    test('unknown extension returns application/octet-stream', () {
      expect(mimeFromExtension('xyz'), 'application/octet-stream');
      expect(mimeFromExtension(null), 'application/octet-stream');
    });
  });

  group('needsConversion', () {
    test('heic needs conversion', () {
      expect(needsConversion('heic'), true);
    });

    test('heif needs conversion', () {
      expect(needsConversion('heif'), true);
    });

    test('case insensitive', () {
      expect(needsConversion('HEIC'), true);
      expect(needsConversion('Heif'), true);
    });

    test('supported formats do not need conversion', () {
      expect(needsConversion('png'), false);
      expect(needsConversion('jpg'), false);
      expect(needsConversion('jpeg'), false);
      expect(needsConversion('gif'), false);
      expect(needsConversion('webp'), false);
    });

    test('null does not need conversion', () {
      expect(needsConversion(null), false);
    });
  });
}
