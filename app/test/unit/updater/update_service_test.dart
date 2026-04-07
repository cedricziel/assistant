import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/updater/update_service.dart';

void main() {
  group('UpdateService.isNewerVersion', () {
    late UpdateService service;

    setUp(() => service = UpdateService());

    test('returns true when latest is strictly greater', () {
      expect(service.isNewerVersion('v1.2.0', '1.1.0'), isTrue);
      expect(service.isNewerVersion('2.0.0', '1.9.9'), isTrue);
      expect(service.isNewerVersion('1.0.1', '1.0.0'), isTrue);
    });

    test('returns false when versions are equal', () {
      expect(service.isNewerVersion('v1.0.0', '1.0.0'), isFalse);
      expect(service.isNewerVersion('1.0.0', 'v1.0.0'), isFalse);
    });

    test('returns false when latest is older', () {
      expect(service.isNewerVersion('1.0.0', '1.1.0'), isFalse);
      expect(service.isNewerVersion('0.9.9', '1.0.0'), isFalse);
    });

    test('returns false for unparseable versions', () {
      expect(service.isNewerVersion('not-a-version', '1.0.0'), isFalse);
      expect(service.isNewerVersion('1.0.0', 'bad'), isFalse);
    });
  });
}
