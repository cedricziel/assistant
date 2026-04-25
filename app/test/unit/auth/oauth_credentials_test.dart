import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/auth/oauth_credentials.dart';

void main() {
  OAuthCredentials makeCreds({
    String accessToken = 'access_123',
    String refreshToken = 'refresh_456',
    DateTime? expiresAt,
    String clientId = 'client_abc',
  }) {
    return OAuthCredentials(
      accessToken: accessToken,
      refreshToken: refreshToken,
      expiresAt: expiresAt ?? DateTime.utc(2026, 6, 1, 12, 0, 0),
      clientId: clientId,
    );
  }

  group('OAuthCredentials', () {
    group('isExpired', () {
      test('returns false when token has not expired', () {
        final creds = makeCreds(
          expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
        );
        expect(creds.isExpired(), isFalse);
      });

      test('returns true when token has expired', () {
        final creds = makeCreds(
          expiresAt: DateTime.now().toUtc().subtract(
            const Duration(minutes: 5),
          ),
        );
        expect(creds.isExpired(), isTrue);
      });

      test('returns true when token expires within default buffer (60s)', () {
        final creds = makeCreds(
          expiresAt: DateTime.now().toUtc().add(const Duration(seconds: 30)),
        );
        expect(creds.isExpired(), isTrue);
      });

      test('respects custom buffer', () {
        final creds = makeCreds(
          expiresAt: DateTime.now().toUtc().add(const Duration(seconds: 30)),
        );
        // With a 10-second buffer, 30 seconds of remaining life is sufficient.
        expect(creds.isExpired(buffer: const Duration(seconds: 10)), isFalse);
      });

      test('returns true at exact expiry time', () {
        final creds = makeCreds(expiresAt: DateTime.now().toUtc());
        expect(creds.isExpired(buffer: Duration.zero), isTrue);
      });
    });

    group('bearerToken', () {
      test('returns the access token', () {
        final creds = makeCreds(accessToken: 'jwt_token_here');
        expect(creds.bearerToken, 'jwt_token_here');
      });
    });

    group('JSON serialization', () {
      test('round-trips through toJson/fromJson', () {
        final original = makeCreds();
        final json = original.toJson();
        final restored = OAuthCredentials.fromJson(json);

        expect(restored.accessToken, original.accessToken);
        expect(restored.refreshToken, original.refreshToken);
        expect(restored.expiresAt, original.expiresAt);
        expect(restored.clientId, original.clientId);
      });

      test('round-trips through toJsonString/fromJsonString', () {
        final original = makeCreds();
        final jsonStr = original.toJsonString();
        final restored = OAuthCredentials.fromJsonString(jsonStr);

        expect(restored, original);
      });

      test('toJson produces expected keys', () {
        final creds = makeCreds();
        final json = creds.toJson();

        expect(json, containsPair('accessToken', 'access_123'));
        expect(json, containsPair('refreshToken', 'refresh_456'));
        expect(json, containsPair('clientId', 'client_abc'));
        expect(json, contains('expiresAt'));
      });

      test('toJsonString produces valid JSON', () {
        final creds = makeCreds();
        final jsonStr = creds.toJsonString();
        final decoded = jsonDecode(jsonStr) as Map<String, dynamic>;

        expect(decoded, isA<Map<String, dynamic>>());
        expect(decoded['accessToken'], 'access_123');
      });
    });

    group('copyWith', () {
      test('copies all fields when none specified', () {
        final original = makeCreds();
        final copy = original.copyWith();

        expect(copy, original);
      });

      test('replaces accessToken', () {
        final original = makeCreds();
        final copy = original.copyWith(accessToken: 'new_token');

        expect(copy.accessToken, 'new_token');
        expect(copy.refreshToken, original.refreshToken);
        expect(copy.clientId, original.clientId);
      });

      test('replaces refreshToken', () {
        final original = makeCreds();
        final copy = original.copyWith(refreshToken: 'new_refresh');

        expect(copy.refreshToken, 'new_refresh');
        expect(copy.accessToken, original.accessToken);
      });

      test('replaces expiresAt', () {
        final original = makeCreds();
        final newExpiry = DateTime.utc(2027, 1, 1);
        final copy = original.copyWith(expiresAt: newExpiry);

        expect(copy.expiresAt, newExpiry);
      });

      test('replaces clientId', () {
        final original = makeCreds();
        final copy = original.copyWith(clientId: 'other_client');

        expect(copy.clientId, 'other_client');
      });
    });

    group('equality', () {
      test('equal when all fields match', () {
        final a = makeCreds();
        final b = makeCreds();

        expect(a, b);
        expect(a.hashCode, b.hashCode);
      });

      test('not equal when accessToken differs', () {
        final a = makeCreds(accessToken: 'a');
        final b = makeCreds(accessToken: 'b');

        expect(a, isNot(b));
      });

      test('not equal when refreshToken differs', () {
        final a = makeCreds(refreshToken: 'a');
        final b = makeCreds(refreshToken: 'b');

        expect(a, isNot(b));
      });

      test('not equal when clientId differs', () {
        final a = makeCreds(clientId: 'a');
        final b = makeCreds(clientId: 'b');

        expect(a, isNot(b));
      });
    });

    group('toString', () {
      test('does not expose tokens', () {
        final creds = makeCreds();
        final str = creds.toString();

        expect(str, contains('clientId'));
        expect(str, contains('expiresAt'));
        expect(str, isNot(contains('access_123')));
        expect(str, isNot(contains('refresh_456')));
      });
    });
  });
}
