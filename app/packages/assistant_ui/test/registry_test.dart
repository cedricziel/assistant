import 'dart:io';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';

final _export = RegExp(r'''^\s*export\s+['"]([^'"]+)['"]''', multiLine: true);
final _widgetClass = RegExp(
  r'^(?:abstract\s+)?(?:final\s+)?class\s+([A-Z]\w*)\s+extends\s+'
  r'(?:StatelessWidget|StatefulWidget)\b',
  multiLine: true,
);
final _adoptedVerdict = RegExp(r'^Adopt(?: \((?:have|scoped)\))?$');

typedef CatalogueRow = ({String verdict, String element});

/// Public widget classes declared in the files that [barrel] exports, minus
/// theme tokens. [readSource] maps a path relative to `lib/` to its source.
Set<String> exportedElements(
  String barrel,
  String Function(String path) readSource,
) {
  final names = <String>{};
  for (final match in _export.allMatches(barrel)) {
    final path = match.group(1)!;
    if (path.startsWith('src/theme/')) continue;
    for (final cls in _widgetClass.allMatches(readSource(path))) {
      names.add(cls.group(1)!);
    }
  }
  return names;
}

/// Numbered catalogue rows of ELEMENTS.md.
List<CatalogueRow> catalogueRows(String markdown) {
  final rows = <CatalogueRow>[];
  for (final line in markdown.split('\n')) {
    if (!line.startsWith('|')) continue;
    final cells = line.split('|').map((c) => c.trim()).toList();
    if (cells.length < 7 || !RegExp(r'^\d+$').hasMatch(cells[1])) continue;
    rows.add((verdict: cells[3].replaceAll('*', ''), element: cells[4]));
  }
  return rows;
}

/// Registry entries that a catalogue row names, but never under an Adopt,
/// Adopt (scoped) or Adopt (have) verdict.
List<String> unadoptedEntries(
  Iterable<String> registry,
  List<CatalogueRow> rows,
) {
  final unadopted = <String>[];
  for (final entry in registry) {
    final mention = RegExp('`$entry(?:\\.\\w+)?`');
    final named = rows.where((r) => mention.hasMatch(r.element)).toList();
    if (named.isNotEmpty &&
        !named.any((r) => _adoptedVerdict.hasMatch(r.verdict))) {
      unadopted.add(entry);
    }
  }
  return unadopted;
}

void main() {
  group('exportedElements', () {
    const sources = {
      'src/a.dart':
          'class Alpha extends StatelessWidget {}\n'
          'class _Hidden extends StatelessWidget {}\n'
          'class Plain {}\n',
      'src/b.dart': 'final class Beta extends StatefulWidget {}\n',
      'src/theme/t.dart': 'class Tokens extends StatelessWidget {}\n',
    };

    test('lists public widget classes and skips theme tokens', () {
      final found = exportedElements(
        "export 'src/a.dart';\nexport 'src/b.dart';\n"
        "export 'src/theme/t.dart';\n",
        (path) => sources[path]!,
      );

      expect(found, {'Alpha', 'Beta'});
    });
  });

  group('unadoptedEntries', () {
    const table = '''
| #  | Element | Verdict | Flutter element | Note |
| -- | ------- | ------- | --------------- | ---- |
| 1  | A | Adopt (have) | `Alpha` | |
| 2  | B | **Adopt (scoped)** | `Beta.slot` | |
| 3  | C | **Decline** | `Gamma` | |
| 4  | D | **Adopt via sibling** | `Delta` | |
''';

    test('flags entries named only by a non-adopt row', () {
      final rows = catalogueRows(table);

      expect(
        unadoptedEntries(['Alpha', 'Beta', 'Gamma', 'Delta', 'Unlisted'], rows),
        ['Gamma', 'Delta'],
      );
    });
  });

  group('kAssistantUiElements', () {
    test('is sorted with no duplicates', () {
      final sorted = kAssistantUiElements.toSet().toList()..sort();

      expect(kAssistantUiElements, sorted);
    });

    test('names every element the barrel exports', () {
      final barrel = File('lib/assistant_ui.dart').readAsStringSync();
      final exported = exportedElements(
        barrel,
        (path) => File('lib/$path').readAsStringSync(),
      );

      expect(
        exported.difference(kAssistantUiElements.toSet()),
        isEmpty,
        reason: 'exported elements missing from kAssistantUiElements',
      );
    });

    test('names no element the catalogue does not adopt', () {
      final rows = catalogueRows(File('ELEMENTS.md').readAsStringSync());

      expect(
        unadoptedEntries(kAssistantUiElements, rows),
        isEmpty,
        reason: 'registered elements without an Adopt row in ELEMENTS.md',
      );
    });
  });
}
