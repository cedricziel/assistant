import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/platform.dart';

void main() {
  group('AppPlatformStyle enum', () {
    test('has exactly three values', () {
      expect(AppPlatformStyle.values.length, 3);
    });

    test('contains cupertino, material, and macos', () {
      expect(
        AppPlatformStyle.values,
        containsAll([
          AppPlatformStyle.cupertino,
          AppPlatformStyle.material,
          AppPlatformStyle.macos,
        ]),
      );
    });
  });

  group('convenience getters', () {
    test('isAppleTouch returns true only for cupertino', () {
      expect(AppPlatformStyle.cupertino.isAppleTouch, isTrue);
      expect(AppPlatformStyle.material.isAppleTouch, isFalse);
      expect(AppPlatformStyle.macos.isAppleTouch, isFalse);
    });

    test('isMacOS returns true only for macos', () {
      expect(AppPlatformStyle.macos.isMacOS, isTrue);
      expect(AppPlatformStyle.cupertino.isMacOS, isFalse);
      expect(AppPlatformStyle.material.isMacOS, isFalse);
    });

    test('isMaterial returns true only for material', () {
      expect(AppPlatformStyle.material.isMaterial, isTrue);
      expect(AppPlatformStyle.cupertino.isMaterial, isFalse);
      expect(AppPlatformStyle.macos.isMaterial, isFalse);
    });
  });

  group('platformStyle getter', () {
    tearDown(() {
      // Reset platform override after each test.
      debugDefaultTargetPlatformOverride = null;
    });

    test('returns macos for TargetPlatform.macOS', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      // platformStyle checks kIsWeb first (false in tests), then
      // defaultTargetPlatform.
      expect(platformStyle, AppPlatformStyle.macos);
    });

    test('returns cupertino for TargetPlatform.iOS', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      expect(platformStyle, AppPlatformStyle.cupertino);
    });

    test('returns material for TargetPlatform.android', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      expect(platformStyle, AppPlatformStyle.material);
    });

    test('returns material for TargetPlatform.linux', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.linux;
      expect(platformStyle, AppPlatformStyle.material);
    });

    test('returns material for TargetPlatform.windows', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.windows;
      expect(platformStyle, AppPlatformStyle.material);
    });
  });

  group('top-level convenience getters', () {
    tearDown(() {
      debugDefaultTargetPlatformOverride = null;
    });

    test('isAppleTouch is true on iOS', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      expect(isAppleTouch, isTrue);
    });

    test('isAppleTouch is false on macOS', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      expect(isAppleTouch, isFalse);
    });

    test('isMaterial is true on Android', () {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      expect(isMaterial, isTrue);
    });
  });
}
