import 'dart:io';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';

/// Purity boundary for `packages/assistant_ui`
/// (openspec/changes/chat-element-library, Decision 2).
///
/// The package renders; the app does state, I/O and platform branching.
const _packageDir = 'packages/assistant_ui';
const _legacyTheme = 'lib/src/theme/legacy_theme.dart';
const _allowedDependencies = {
  'flutter',
  'flutter_smooth_markdown',
  'material_ui',
  'highlight',
};
const _highlightDir = 'lib/src/highlight/';
const _clipboardAndAnnounce = ['Clipboard.', 'SemanticsService.'];
const _forbiddenImports = [
  'flutter_riverpod',
  'assistant_api',
  'cupertino_ui',
  'package:flutter/cupertino.dart',
  'dart:io',
  'record',
  'file_picker',
  'desktop_drop',
  'url_launcher',
  'audioplayers',
  'http',
  'cached_network_image',
  'flutter_svg',
  'share_plus',
  'path_provider',
];
const _forbiddenMaterial = 'package:flutter/material.dart';
const _platformBranching = [
  'defaultTargetPlatform',
  '.platform ==',
  'TargetPlatform.',
];

final _directive = RegExp(
  r'''^\s*(?:import|export)\s+['"]([^'"]+)['"]''',
  multiLine: true,
);

const _chatFeatureDir = 'lib/features/chat';

final _appWidgetClass = RegExp(
  r'^(?:abstract\s+)?(?:final\s+)?class\s+([A-Z]\w*)\s+extends\s+'
  r'(?:StatelessWidget|StatefulWidget|ConsumerWidget|ConsumerStatefulWidget)\b',
  multiLine: true,
);

/// Widgets in the app's chat feature whose name reuses an element's name.
///
/// Adapters compose a package element under a distinct name, so a shared name
/// means render code is still living in the app.
List<String> duplicateElementWidgets(
  Map<String, String> sources,
  Iterable<String> elements,
) {
  final registry = elements.toSet();
  final duplicates = <String>[];
  for (final MapEntry(key: path, value: source) in sources.entries) {
    for (final match in _appWidgetClass.allMatches(source)) {
      final name = match.group(1)!;
      if (registry.contains(name)) duplicates.add('$path: $name');
    }
  }
  return duplicates;
}

/// Top-level keys of `dependencies:` in a pubspec, without a YAML parser.
Set<String> dependencyNames(String pubspec) {
  final names = <String>{};
  var inDependencies = false;
  for (final line in pubspec.split('\n')) {
    if (line.trim().isEmpty || line.trimLeft().startsWith('#')) continue;
    if (!line.startsWith(' ')) {
      inDependencies = line.trimRight() == 'dependencies:';
    } else if (inDependencies) {
      final match = RegExp(r'^ {2}([\w.]+):').firstMatch(line);
      if (match != null) names.add(match.group(1)!);
    }
  }
  return names;
}

bool _references(String uri, String forbidden) =>
    uri == forbidden ||
    uri == 'dart:$forbidden' ||
    uri.startsWith('package:$forbidden/');

/// Boundary violations for [sources], keyed by path relative to the package.
List<String> boundaryViolations(Map<String, String> sources) {
  final violations = <String>[];
  for (final MapEntry(key: path, value: source) in sources.entries) {
    for (final uri in _directive.allMatches(source).map((m) => m.group(1)!)) {
      for (final forbidden in _forbiddenImports) {
        if (_references(uri, forbidden)) {
          violations.add('$path: forbidden import $uri');
        }
      }
      if (uri == _forbiddenMaterial && path != _legacyTheme) {
        violations.add('$path: $uri outside $_legacyTheme');
      }
      if (uri.startsWith('package:highlight/') &&
          !path.startsWith(_highlightDir)) {
        violations.add('$path: $uri outside $_highlightDir');
      }
    }
    for (final token in _platformBranching) {
      if (source.contains(token)) {
        violations.add('$path: platform branching via $token');
      }
    }
    for (final token in _clipboardAndAnnounce) {
      if (source.contains(token)) {
        violations.add('$path: $token is the app\'s job, not the package\'s');
      }
    }
  }
  return violations;
}

void main() {
  group('assistant_ui boundary', () {
    test('dependencies are exactly the allowlist', () {
      final pubspec = File('$_packageDir/pubspec.yaml').readAsStringSync();

      expect(dependencyNames(pubspec), _allowedDependencies);
    });

    test('package source respects the purity boundary', () {
      final sources = {
        for (final file
            in Directory('$_packageDir/lib')
                .listSync(recursive: true)
                .whereType<File>()
                .where((f) => f.path.endsWith('.dart')))
          file.path.substring(_packageDir.length + 1): file.readAsStringSync(),
      };

      expect(sources, isNotEmpty);
      expect(boundaryViolations(sources), isEmpty);
    });

    test(
      'chat widgets carry distinct names from the elements they compose',
      () {
        final sources = {
          for (final file
              in Directory(_chatFeatureDir)
                  .listSync(recursive: true)
                  .whereType<File>()
                  .where((f) => f.path.endsWith('.dart')))
            file.path.substring('lib/'.length): file.readAsStringSync(),
        };

        expect(sources, isNotEmpty);
        expect(
          duplicateElementWidgets(sources, kAssistantUiElements),
          isEmpty,
          reason: 'chat adapters must not reuse an exported element name',
        );
      },
    );
  });

  group('duplicateElementWidgets', () {
    test('flags widget classes that share an element name', () {
      final duplicates = duplicateElementWidgets(
        {
          'lib/features/chat/a.dart':
              'class MessageBubble extends StatelessWidget {}\n'
              'class ChatMessageBubbleAdapter extends StatelessWidget {}\n',
          'lib/features/chat/b.dart':
              'class _Private extends StatefulWidget {}\n'
              'class Plain {\n',
        },
        ['MessageBubble', 'Composer'],
      );

      expect(duplicates, ['lib/features/chat/a.dart: MessageBubble']);
    });
  });

  group('boundaryViolations', () {
    test('flags forbidden imports, exports and prefixed packages', () {
      final violations = boundaryViolations({
        'lib/a.dart':
            "import 'package:flutter_riverpod/flutter_riverpod.dart';",
        'lib/b.dart': 'export "package:http/http.dart";',
        'lib/c.dart':
            "import 'dart:io';\nimport 'package:flutter/cupertino.dart';",
      });

      expect(violations, hasLength(4));
    });

    test('flags material.dart outside the legacy theme file only', () {
      const material = "import 'package:flutter/material.dart';";

      expect(boundaryViolations({_legacyTheme: material}), isEmpty);
      expect(boundaryViolations({'lib/src/x.dart': material}), hasLength(1));
    });

    test('flags platform branching but allows .adaptive constructors', () {
      final violations = boundaryViolations({
        'lib/a.dart': 'if (defaultTargetPlatform == TargetPlatform.iOS) {}',
        'lib/b.dart': 'if (theme.platform == x) {}',
        'lib/c.dart': 'const CircularProgressIndicator.adaptive();',
      });

      expect(violations, hasLength(3));
      expect(violations.any((v) => v.startsWith('lib/c.dart')), isFalse);
    });

    test('allows package:highlight only under lib/src/highlight/', () {
      const import = "import 'package:highlight/highlight.dart';";

      expect(boundaryViolations({'lib/src/highlight/h.dart': import}), isEmpty);
      expect(boundaryViolations({'lib/src/x.dart': import}), hasLength(1));
    });

    test('flags clipboard and semantics announcements', () {
      final violations = boundaryViolations({
        'lib/a.dart': 'Clipboard.setData(x);',
        'lib/b.dart': 'SemanticsService.sendAnnouncement(v, m, d);',
      });

      expect(violations, hasLength(2));
    });

    test('does not confuse similarly named packages', () {
      expect(
        boundaryViolations({
          'lib/a.dart': "import 'package:http_parser/http_parser.dart';",
        }),
        isEmpty,
      );
    });

    test('dependencyNames ignores dev_dependencies and nested keys', () {
      const pubspec = '''
name: x
dependencies:
  flutter:
    sdk: flutter
  material_ui: ^1.0.0
dev_dependencies:
  flutter_test:
    sdk: flutter
''';

      expect(dependencyNames(pubspec), {'flutter', 'material_ui'});
    });
  });
}
