import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/auth/pkce.dart';

void main() {
  group('PkceChallenge', () {
    group('generate', () {
      test('produces a non-empty verifier', () {
        final pkce = PkceChallenge.generate();
        expect(pkce.verifier, isNotEmpty);
      });

      test('produces a non-empty challenge', () {
        final pkce = PkceChallenge.generate();
        expect(pkce.challenge, isNotEmpty);
      });

      test('verifier is base64url-encoded without padding', () {
        final pkce = PkceChallenge.generate();
        expect(pkce.verifier, isNot(contains('=')));
        // base64url alphabet: A-Z, a-z, 0-9, -, _
        expect(pkce.verifier, matches(RegExp(r'^[A-Za-z0-9_-]+$')));
      });

      test('challenge is base64url-encoded without padding', () {
        final pkce = PkceChallenge.generate();
        expect(pkce.challenge, isNot(contains('=')));
        expect(pkce.challenge, matches(RegExp(r'^[A-Za-z0-9_-]+$')));
      });

      test(
        'default verifier has expected length (~43 chars from 32 bytes)',
        () {
          final pkce = PkceChallenge.generate();
          // 32 bytes → ceil(32*4/3) = 43 base64 chars (no padding)
          expect(pkce.verifier.length, 43);
        },
      );

      test('challenge is always 43 chars (SHA-256 → 32 bytes → base64url)', () {
        final pkce = PkceChallenge.generate();
        expect(pkce.challenge.length, 43);
      });

      test('custom length produces different verifier length', () {
        final pkce = PkceChallenge.generate(length: 64);
        // 64 bytes → ceil(64*4/3) = 86 base64 chars (no padding)
        expect(pkce.verifier.length, 86);
        // challenge is still 43 (always SHA-256 output)
        expect(pkce.challenge.length, 43);
      });

      test('generates unique verifiers on each call', () {
        final a = PkceChallenge.generate();
        final b = PkceChallenge.generate();
        expect(a.verifier, isNot(a.challenge));
        expect(a.verifier, isNot(b.verifier));
      });

      test('generates unique challenges on each call', () {
        final a = PkceChallenge.generate();
        final b = PkceChallenge.generate();
        expect(a.challenge, isNot(b.challenge));
      });
    });

    group('S256 correctness', () {
      test('challenge equals BASE64URL(SHA256(verifier))', () {
        final pkce = PkceChallenge.generate();

        // Manually compute expected challenge.
        final digest = sha256.convert(utf8.encode(pkce.verifier));
        final expected = base64UrlEncode(digest.bytes).replaceAll('=', '');

        expect(pkce.challenge, expected);
      });

      test('deriveChallenge matches generated challenge', () {
        final pkce = PkceChallenge.generate();
        final derived = PkceChallenge.deriveChallenge(pkce.verifier);
        expect(derived, pkce.challenge);
      });

      test('known test vector', () {
        // RFC 7636 Appendix B test vector:
        // verifier: dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk
        // expected challenge: E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM
        const verifier = 'dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk';
        const expected = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';

        final challenge = PkceChallenge.deriveChallenge(verifier);
        expect(challenge, expected);
      });
    });

    group('toString', () {
      test('includes truncated verifier', () {
        final pkce = PkceChallenge.generate();
        final str = pkce.toString();
        expect(str, contains('PkceChallenge'));
        expect(str, contains('...'));
        // Should not contain full verifier
        expect(str, isNot(contains(pkce.verifier)));
      });
    });
  });
}
