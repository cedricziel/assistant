import 'package:flutter_test/flutter_test.dart';

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
  });
}
