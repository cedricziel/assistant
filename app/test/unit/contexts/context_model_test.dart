import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';

void main() {
  group('AssistantContext', () {
    final fixed = DateTime.utc(2024, 1, 15, 12, 0, 0);

    AssistantContext makeCtx({
      String id = 'test-id',
      String name = 'Work',
      String serverUrl = 'https://work.example.com',
      String? authToken,
    }) {
      return AssistantContext(
        id: id,
        name: name,
        serverUrl: serverUrl,
        authToken: authToken,
        createdAt: fixed,
      );
    }

    // -- serialisation round-trip --------------------------------------------

    test('toJson omits authToken', () {
      final ctx = makeCtx(authToken: 'secret');
      final json = ctx.toJson();
      expect(json.containsKey('authToken'), isFalse);
    });

    test('toJson includes required fields', () {
      final ctx = makeCtx();
      final json = ctx.toJson();
      expect(json['id'], 'test-id');
      expect(json['name'], 'Work');
      expect(json['serverUrl'], 'https://work.example.com');
      expect(json['createdAt'], fixed.toIso8601String());
    });

    test('fromJson round-trip restores fields', () {
      final original = makeCtx();
      final restored = AssistantContext.fromJson(original.toJson());
      expect(restored.id, original.id);
      expect(restored.name, original.name);
      expect(restored.serverUrl, original.serverUrl);
      expect(restored.createdAt, original.createdAt);
      expect(restored.authToken, isNull);
    });

    test('toJsonString / fromJsonString round-trip', () {
      final original = makeCtx();
      final restored = AssistantContext.fromJsonString(original.toJsonString());
      expect(restored, equals(original));
    });

    // -- copyWith ------------------------------------------------------------

    test('copyWith updates name', () {
      final ctx = makeCtx();
      final updated = ctx.copyWith(name: 'Personal');
      expect(updated.name, 'Personal');
      expect(updated.id, ctx.id);
    });

    test('copyWith with clearAuthToken removes token', () {
      final ctx = makeCtx(authToken: 'my-token');
      final cleared = ctx.copyWith(clearAuthToken: true);
      expect(cleared.authToken, isNull);
    });

    test('copyWith without clearAuthToken preserves token', () {
      final ctx = makeCtx(authToken: 'my-token');
      final copy = ctx.copyWith(name: 'Other');
      expect(copy.authToken, 'my-token');
    });

    // -- equality & identity -------------------------------------------------

    test('equality is based on id only', () {
      final a = makeCtx(id: 'abc');
      final b = AssistantContext(
        id: 'abc',
        name: 'Different',
        serverUrl: 'http://other.com',
        createdAt: DateTime.now(),
      );
      expect(a, equals(b));
    });

    test('different ids are not equal', () {
      final a = makeCtx(id: 'aaa');
      final b = makeCtx(id: 'bbb');
      expect(a, isNot(equals(b)));
    });

    // -- factory create ------------------------------------------------------

    test('create() generates a non-empty UUID', () {
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
      );
      expect(ctx.id, isNotEmpty);
      expect(ctx.id.length, greaterThan(10));
    });

    test('create() sets createdAt to a recent UTC time', () {
      final before = DateTime.now().toUtc().subtract(
        const Duration(seconds: 1),
      );
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
      );
      final after = DateTime.now().toUtc().add(const Duration(seconds: 1));
      expect(ctx.createdAt.isAfter(before), isTrue);
      expect(ctx.createdAt.isBefore(after), isTrue);
    });

    // -- AuthMode ---------------------------------------------------------------

    test('default authMode is legacyToken', () {
      final ctx = makeCtx();
      expect(ctx.authMode, AuthMode.legacyToken);
    });

    test('create() accepts authMode parameter', () {
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
        authMode: AuthMode.oauth2,
      );
      expect(ctx.authMode, AuthMode.oauth2);
    });

    test('toJson includes authMode', () {
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
        authMode: AuthMode.oauth2,
      );
      expect(ctx.toJson()['authMode'], 'oauth2');
    });

    test('fromJson restores authMode', () {
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
        authMode: AuthMode.oauth2,
      );
      final restored = AssistantContext.fromJson(ctx.toJson());
      expect(restored.authMode, AuthMode.oauth2);
    });

    test('fromJson defaults to legacyToken when authMode is missing', () {
      final json = {
        'id': 'old-ctx',
        'name': 'Old',
        'serverUrl': 'http://old.example.com',
        'createdAt': DateTime.now().toUtc().toIso8601String(),
      };
      final restored = AssistantContext.fromJson(json);
      expect(restored.authMode, AuthMode.legacyToken);
    });

    test('toJson omits oauthCredentials', () {
      final creds = OAuthCredentials(
        accessToken: 'at',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc(),
        clientId: 'cid',
      );
      final ctx = AssistantContext.create(
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
      );
      final json = ctx.toJson();
      expect(json.containsKey('oauthCredentials'), isFalse);
    });

    // -- effectiveToken ---------------------------------------------------------

    test('effectiveToken returns authToken for legacyToken mode', () {
      final ctx = makeCtx(authToken: 'legacy-tok');
      expect(ctx.effectiveToken, 'legacy-tok');
    });

    test('effectiveToken returns null when legacyToken has no token', () {
      final ctx = makeCtx();
      expect(ctx.effectiveToken, isNull);
    });

    test('effectiveToken returns accessToken for oauth2 mode', () {
      final creds = OAuthCredentials(
        accessToken: 'oauth-access',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
        clientId: 'cid',
      );
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      expect(ctx.effectiveToken, 'oauth-access');
    });

    test('effectiveToken falls back to authToken when oauth2 has no creds', () {
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        authToken: 'fallback',
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      expect(ctx.effectiveToken, 'fallback');
    });

    // -- needsTokenRefresh ------------------------------------------------------

    test('needsTokenRefresh is false for legacyToken mode', () {
      final ctx = makeCtx(authToken: 'tok');
      expect(ctx.needsTokenRefresh, isFalse);
    });

    test('needsTokenRefresh is false when oauth2 creds are not expired', () {
      final creds = OAuthCredentials(
        accessToken: 'at',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
        clientId: 'cid',
      );
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      expect(ctx.needsTokenRefresh, isFalse);
    });

    test('needsTokenRefresh is true when oauth2 creds are expired', () {
      final creds = OAuthCredentials(
        accessToken: 'at',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc().subtract(const Duration(hours: 1)),
        clientId: 'cid',
      );
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      expect(ctx.needsTokenRefresh, isTrue);
    });

    test('needsTokenRefresh is false when oauth2 has no creds', () {
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      expect(ctx.needsTokenRefresh, isFalse);
    });

    // -- copyWith with OAuth fields -------------------------------------------

    test('copyWith preserves oauthCredentials', () {
      final creds = OAuthCredentials(
        accessToken: 'at',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc(),
        clientId: 'cid',
      );
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      final copy = ctx.copyWith(name: 'Updated');
      expect(copy.oauthCredentials, creds);
      expect(copy.authMode, AuthMode.oauth2);
    });

    test('copyWith with clearOAuthCredentials removes creds', () {
      final creds = OAuthCredentials(
        accessToken: 'at',
        refreshToken: 'rt',
        expiresAt: DateTime.now().toUtc(),
        clientId: 'cid',
      );
      final ctx = AssistantContext(
        id: 'test',
        name: 'Test',
        serverUrl: 'http://localhost',
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
        createdAt: DateTime.now().toUtc(),
      );
      final cleared = ctx.copyWith(clearOAuthCredentials: true);
      expect(cleared.oauthCredentials, isNull);
    });

    test('copyWith can change authMode', () {
      final ctx = makeCtx();
      final updated = ctx.copyWith(authMode: AuthMode.oauth2);
      expect(updated.authMode, AuthMode.oauth2);
    });
  });
}
