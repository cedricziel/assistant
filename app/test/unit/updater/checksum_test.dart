import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SHA-256 checksum verification', () {
    late Directory tempDir;
    late File artifact;

    setUp(() async {
      tempDir = await Directory.systemTemp.createTemp('checksum_test_');
      artifact = File('${tempDir.path}/artifact.bin');
      await artifact.writeAsBytes(utf8.encode('hello world'));
    });

    tearDown(() async {
      await tempDir.delete(recursive: true);
    });

    test('matching checksum passes verification', () async {
      final bytes = await artifact.readAsBytes();
      final actual = sha256.convert(bytes).toString();
      final checksumMap = {'artifact.bin': actual};

      final expected = checksumMap['artifact.bin'];
      final computed = sha256.convert(bytes).toString();
      expect(computed, equals(expected));
    });

    test('mismatched checksum fails verification', () async {
      final bytes = await artifact.readAsBytes();
      const wrong = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef';
      final checksumMap = {'artifact.bin': wrong};

      final expected = checksumMap['artifact.bin'];
      final computed = sha256.convert(bytes).toString();
      expect(computed, isNot(equals(expected)));
    });

    test('missing filename in checksum file fails gracefully', () {
      final checksumMap = <String, String>{};
      final expected = checksumMap['artifact.bin'];
      expect(expected, isNull);
    });
  });
}
