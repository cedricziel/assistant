import 'dart:convert';
import 'dart:io';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';

/// Kinds whose view has not landed yet. C4–C7 each remove their kinds until
/// this set is empty; a kind outside it without a renderer fails the test.
const pendingKinds = <String>{'terminal', 'sources'};

/// `schemas/tool-results/fixtures`, resolved from the package root. Fails when
/// it is missing or holds no fixtures, so a wrong path cannot pass with zero
/// cases.
Directory fixturesDir() {
  final dir = Directory('../../../schemas/tool-results/fixtures');
  if (!dir.existsSync()) {
    fail('fixture directory not found: ${dir.absolute.path}');
  }
  final hasFixtures = dir
      .listSync(recursive: true)
      .any((e) => e is File && e.path.endsWith('.json'));
  if (!hasFixtures) fail('no fixtures under ${dir.absolute.path}');
  return dir;
}

/// Fixture files of [kind], by file name.
Map<String, Map<String, Object?>> fixturesOf(Directory dir, String kind) => {
  for (final file in Directory('${dir.path}/$kind').listSync())
    if (file is File && file.path.endsWith('.json'))
      file.uri.pathSegments.last: Map<String, Object?>.from(
        jsonDecode(file.readAsStringSync()) as Map,
      ),
};

void main() {
  final dir = fixturesDir();
  final kinds = [
    for (final entry in dir.listSync())
      if (entry is Directory) entry.uri.pathSegments.reversed.skip(1).first,
  ]..sort();
  final renderers = {
    for (final renderer in defaultToolResultRenderers())
      renderer.kind: renderer,
  };

  test('every fixture kind is known', () {
    expect(kinds, isNotEmpty);
    expect(
      pendingKinds.difference(kinds.toSet()),
      isEmpty,
      reason: 'pendingKinds names a kind with no fixtures',
    );
  });

  for (final kind in kinds) {
    group(kind, () {
      test('has a renderer or is pending', () {
        expect(
          renderers.containsKey(kind) || pendingKinds.contains(kind),
          isTrue,
          reason: 'no renderer registered for kind "$kind"',
        );
      });

      test('pending kinds have no renderer yet', () {
        if (pendingKinds.contains(kind)) {
          expect(renderers, isNot(contains(kind)));
        }
      });

      final renderer = renderers[kind];
      if (renderer == null) return;
      fixturesOf(dir, kind).forEach((name, data) {
        final valid = name.startsWith('valid-');
        test('$name ${valid ? 'parses' : 'is rejected'}', () {
          expect(name, matches(RegExp('^(valid|invalid)-')));
          expect(renderer.parse(data), valid ? isNotNull : isNull);
        });
      });
    });
  }
}
